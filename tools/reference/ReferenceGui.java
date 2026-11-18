package biogenesis;

import java.awt.*;
import java.awt.datatransfer.StringSelection;
import java.awt.event.InputEvent;
import java.awt.event.KeyEvent;
import java.io.ByteArrayOutputStream;
import java.io.ObjectOutputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.concurrent.Callable;
import java.util.concurrent.FutureTask;
import java.util.prefs.Preferences;
import javax.imageio.ImageIO;
import javax.swing.*;

/** Exercise real reference controls on a caller-owned virtual display */
public final class ReferenceGui {
    private static Robot robot;
    private static MainWindow window;
    private static Path output;

    private static void require(boolean condition, String message) {
        if (!condition) throw new IllegalStateException(message);
    }

    private static <T> T ui(Callable<T> work) throws Exception {
        FutureTask<T> task = new FutureTask<>(work);
        SwingUtilities.invokeAndWait(task);
        return task.get();
    }

    private static void observe(String event) throws Exception {
        String record = ui(() -> "{\"event\":\"" + event + "\",\"population\":" + window.getWorld().getPopulation()
                + ",\"time\":" + window.getWorld().getTime() + ",\"frame\":" + window.getWorld().getFrame()
                + ",\"active\":" + window.isProcessActive() + ",\"co2\":\""
                + Double.toHexString(window.getWorld().getCO2()) + "\"}");
        System.out.println(record);
        System.out.flush();
    }

    private static long ticks() throws Exception {
        return ui(() -> window.getWorld().getTime() * 256L + window.getWorld().getFrame());
    }

    private static void click(Component component) throws Exception {
        Point point = ui(() -> {
            require(component.isShowing(), "Control is not visible");
            Point location = component.getLocationOnScreen();
            return new Point(location.x + component.getWidth() / 2, location.y + component.getHeight() / 2);
        });
        robot.mouseMove(point.x, point.y);
        robot.mousePress(InputEvent.BUTTON1_DOWN_MASK);
        robot.mouseRelease(InputEvent.BUTTON1_DOWN_MASK);
        robot.waitForIdle();
        robot.delay(150);
    }

    private static AbstractButton action_button(Container container, Action action) {
        for (Component component : container.getComponents()) {
            if (component instanceof AbstractButton button && button.getAction() == action && button.isShowing()) return button;
            if (component instanceof Container child) {
                AbstractButton found = action_button(child, action);
                if (found != null) return found;
            }
        }
        return null;
    }

    private static void toolbar(Action action) throws Exception {
        AbstractButton button = ui(() -> action_button(window.getContentPane(), action));
        require(button != null, "Missing toolbar control");
        click(button);
    }

    private static void menu(Action action) throws Exception {
        JMenu parent = ui(() -> {
            for (int index = 0; index < window.getJMenuBar().getMenuCount(); index++) {
                JMenu entry = window.getJMenuBar().getMenu(index);
                for (Component component : entry.getMenuComponents()) {
                    if (component instanceof JMenuItem item && item.getAction() == action) return entry;
                }
            }
            throw new IllegalStateException("Missing menu action");
        });
        click(parent);
        JMenuItem item = ui(() -> {
            for (Component component : parent.getMenuComponents()) {
                if (component instanceof JMenuItem candidate && candidate.getAction() == action) return candidate;
            }
            throw new IllegalStateException("Missing opened menu item");
        });
        click(item);
    }

    private static AbstractButton text_button(Container container, String text) {
        for (Component component : container.getComponents()) {
            if (component instanceof AbstractButton button && text.equals(button.getText()) && button.isShowing()) return button;
            if (component instanceof Container child) {
                AbstractButton found = text_button(child, text);
                if (found != null) return found;
            }
        }
        return null;
    }

    private static Window dialog() throws Exception {
        return ui(() -> {
            for (Window candidate : Window.getWindows()) {
                if (candidate != window && candidate.isShowing()) return candidate;
            }
            throw new IllegalStateException("No visible dialog");
        });
    }

    private static void close_dialog(String label) throws Exception {
        Window target = dialog();
        AbstractButton button = ui(() -> text_button(target, label));
        require(button != null, "Missing dialog button: " + label);
        click(button);
        require(ui(() -> !target.isShowing()), "Dialog did not close");
    }

    private static void capture(String name) throws Exception {
        robot.waitForIdle();
        Rectangle bounds = new Rectangle(Toolkit.getDefaultToolkit().getScreenSize());
        require(ImageIO.write(robot.createScreenCapture(bounds), "png", output.resolve(name + ".png").toFile()), "PNG writer unavailable");
    }

    private static void key(int key) {
        robot.keyPress(key);
        robot.keyRelease(key);
    }

    private static void choose_file(Path file) throws Exception {
        Toolkit.getDefaultToolkit().getSystemClipboard().setContents(new StringSelection(file.toString()), null);
        robot.keyPress(KeyEvent.VK_CONTROL);
        key(KeyEvent.VK_A);
        robot.keyRelease(KeyEvent.VK_CONTROL);
        robot.keyPress(KeyEvent.VK_CONTROL);
        key(KeyEvent.VK_V);
        robot.keyRelease(KeyEvent.VK_CONTROL);
        key(KeyEvent.VK_ENTER);
        robot.waitForIdle();
        robot.delay(200);
    }

    private static Path save_world() throws Exception {
        toolbar(window.saveGameAction);
        Path save = output.resolve("control-save.bgw");
        choose_file(save);
        for (int attempt = 0; attempt < 20 && !Files.exists(save); attempt++) robot.delay(100);
        require(Files.isRegularFile(save) && Files.size(save) > 0, "Real save control did not write the selected file");
        observe("saved-through-controls");
        return save;
    }

    private static void startup_snapshot() throws Exception {
        byte[] bytes = ui(() -> {
            ByteArrayOutputStream buffer = new ByteArrayOutputStream();
            try (ObjectOutputStream stream = new ObjectOutputStream(buffer)) {
                stream.writeObject(window.getWorld());
            }
            return buffer.toByteArray();
        });
        Files.write(output.resolve("startup-world.bgw"), bytes, StandardOpenOption.CREATE_NEW);
        String hash = HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
        System.out.println("{\"event\":\"startup-snapshot\",\"sha256\":\"" + hash + "\",\"bytes\":" + bytes.length + "}");
    }

    private static void run() throws Exception {
        Preferences preferences = Preferences.userNodeForPackage(Utils.class);
        preferences.putInt("VERSION", 700);
        preferences.putInt("INITIAL_ORGANISMS", 32);
        preferences.putInt("WORLD_WIDTH", 512);
        preferences.putInt("WORLD_HEIGHT", 400);
        preferences.putDouble("INITIAL_CO2", 100000);
        preferences.putDouble("INITIAL_O2", 5000);
        preferences.putInt("WINDOW_WIDTH", 900);
        preferences.putInt("WINDOW_HEIGHT", 700);
        preferences.putInt("HARDWARE_ACCELERATION", 0);
        preferences.putInt("DELAY", 20);
        preferences.putBoolean("ACCEPT_CONNECTIONS", false);
        preferences.putBoolean("CONNECT_TO_SERVER", false);
        preferences.putBoolean("AUTO_BACKUP", false);
        preferences.put("LOCALE", "en");
        preferences.flush();
        ui(() -> { MainWindow.main(new String[]{"827364"}); return null; });
        window = ui(() -> {
            for (Frame frame : Frame.getFrames()) if (frame instanceof MainWindow main) return main;
            throw new IllegalStateException("Reference entry point did not create its main window");
        });
        robot = new Robot();
        robot.setAutoDelay(60);
        robot.waitForIdle();
        require(ui(() -> window.isShowing() && !window.isProcessActive()), "Reference did not start visible and paused");
        require(ui(() -> window.getWorld().getPopulation()) > 0, "Seeded genesis produced no organisms");
        capture("startup");
        observe("startup-paused");
        startup_snapshot();
        long before = ticks();
        toolbar(window.startStopAction);
        robot.delay(500);
        require(ui(() -> window.isProcessActive()) && ticks() > before, "Resume control did not advance simulation");
        capture("running");
        toolbar(window.startStopAction);
        long paused = ticks();
        robot.delay(250);
        require(!ui(() -> window.isProcessActive()) && ticks() == paused, "Pause control did not stop simulation");
        observe("resumed-and-paused");
        double gas = ui(() -> window.getWorld().getCO2());
        toolbar(window.increaseCO2Action);
        require(ui(() -> window.getWorld().getCO2()) == gas + 1000, "Gas control changed an unexpected quantity");
        toolbar(window.decreaseCO2Action);
        require(ui(() -> window.getWorld().getCO2()) == gas, "Gas controls did not reverse");
        menu(window.labAction);
        capture("laboratory");
        close_dialog(Messages.getString("T_CANCEL"));
        menu(window.parametersAction);
        capture("parameters");
        close_dialog(Messages.getString("T_CANCEL"));
        long saved_ticks = ticks();
        Path save = save_world();
        capture("saved-world");
        menu(window.newGameAction);
        close_dialog(Messages.getString("T_YES"));
        require(!ui(() -> window.isProcessActive()) && ticks() == 0, "New-world control did not reset and pause");
        observe("new-world-paused");
        menu(window.openGameAction);
        choose_file(save);
        require(!ui(() -> window.isProcessActive()) && ticks() == saved_ticks, "Load control did not restore saved time and pause");
        require(ui(() -> window.getWorld().getCO2()) == gas, "Load control did not restore saved gas state");
        observe("loaded-through-controls");
        capture("loaded-world");
        menu(window.quitAction);
        close_dialog(Messages.getString("T_CANCEL"));
        require(ui(() -> window.isShowing()), "Quit cancellation closed the application");
        observe("quit-cancelled");
        menu(window.quitAction);
        Window confirmation = dialog();
        AbstractButton no = ui(() -> text_button(confirmation, Messages.getString("T_NO")));
        require(no != null, "Missing quit-without-save choice");
        observe("quit-without-save-requested");
        click(no);
        robot.delay(1500);
        throw new IllegalStateException("Reference quit control did not exit");
    }

    public static void main(String[] args) throws Exception {
        String display = System.getenv("DISPLAY");
        require("1".equals(System.getenv("BGR_ISOLATED_XVFB")) && display != null && !display.equals(":0"),
                "Run only on the dedicated reference Xvfb display");
        require(args.length == 1, "Expected isolated output directory");
        output = Path.of(args[0]).toAbsolutePath();
        Files.createDirectories(output);
        try (var files = Files.list(output)) {
            require(files.findAny().isEmpty(), "Output directory must be empty");
        }
        Path run_root = output.getParent().toRealPath();
        for (String property : new String[]{"user.home", "java.io.tmpdir", "java.util.prefs.userRoot", "java.util.prefs.systemRoot"}) {
            String value = System.getProperty(property);
            require(value != null, "Missing isolated profile property: " + property);
            Path profile = Path.of(value).toRealPath();
            require(profile.startsWith(run_root) && !profile.equals(run_root), "Profile must stay below the fresh run root: " + property);
        }
        try {
            run();
        } catch (Throwable error) {
            if (robot != null) {
                try {
                    capture("failure");
                } catch (Throwable capture_error) {
                    error.addSuppressed(capture_error);
                }
            }
            try {
                ui(() -> {
                    for (Frame frame : Frame.getFrames()) {
                        if (frame instanceof MainWindow main && main._timer != null) main._timer.cancel();
                    }
                    for (Window target : Window.getWindows()) target.dispose();
                    return null;
                });
            } catch (Throwable cleanup_error) {
                error.addSuppressed(cleanup_error);
            } finally {
                error.printStackTrace();
                System.exit(1);
            }
        }
    }
}
