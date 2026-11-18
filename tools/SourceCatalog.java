import com.sun.source.tree.*;
import com.sun.source.util.JavacTask;
import com.sun.source.util.TreePath;
import com.sun.source.util.TreePathScanner;
import com.sun.source.util.Trees;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import java.util.Locale;
import javax.tools.Diagnostic;
import javax.tools.DiagnosticCollector;
import javax.tools.JavaFileObject;
import javax.tools.ToolProvider;

/** Parse source declarations and decisions without analyzing or executing reference code */
public final class SourceCatalog {
    private static String quote(String value) {
        StringBuilder out = new StringBuilder("\"");
        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            switch (c) {
                case '"' -> out.append("\\\"");
                case '\\' -> out.append("\\\\");
                case '\n' -> out.append("\\n");
                case '\r' -> out.append("\\r");
                case '\t' -> out.append("\\t");
                default -> {
                    if (c < 32) out.append(String.format(Locale.ROOT, "\\u%04x", (int)c));
                    else out.append(c);
                }
            }
        }
        return out.append('"').toString();
    }

    private static String digest(String value) {
        try {
            return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(value.getBytes(StandardCharsets.UTF_8)));
        } catch (java.security.NoSuchAlgorithmException error) {
            throw new AssertionError(error);
        }
    }

    public static void main(String[] args) throws Exception {
        if (args.length < 2) throw new IllegalArgumentException("Expected source root and Java source paths");
        Path root = Path.of(args[0]).toAbsolutePath().normalize();
        var compiler = ToolProvider.getSystemJavaCompiler();
        if (compiler == null) throw new IllegalStateException("A JDK is required");
        var diagnostics = new DiagnosticCollector<JavaFileObject>();
        List<String> output = new ArrayList<>();
        try (var manager = compiler.getStandardFileManager(diagnostics, Locale.ROOT, StandardCharsets.UTF_8)) {
            List<Path> paths = new ArrayList<>();
            for (int i = 1; i < args.length; i++) paths.add(Path.of(args[i]));
            var task = (JavacTask)compiler.getTask(null, manager, diagnostics, List.of("-proc:none"), null,
                    manager.getJavaFileObjectsFromPaths(paths));
            var positions = Trees.instance(task).getSourcePositions();
            for (var unit : task.parse()) {
                Path path = Path.of(unit.getSourceFile().toUri()).toAbsolutePath().normalize();
                String file = root.relativize(path).toString().replace('\\', '/');
                String source = Files.readString(path, StandardCharsets.UTF_8);
                new TreePathScanner<Void, Void>() {
                    String class_name = "";
                    String method_name = "";

                    void emit(Tree node, String kind, String name, String expression) {
                        long start = positions.getStartPosition(unit, node);
                        long end = positions.getEndPosition(unit, node);
                        if (start < 0 || end < start) throw new IllegalStateException("Missing source position: " + node.getKind());
                        String body = source.substring(Math.toIntExact(start), Math.toIntExact(end));
                        output.add("{\"file\":" + quote(file) + ",\"kind\":" + quote(kind)
                                + ",\"name\":" + quote(name) + ",\"class\":" + quote(class_name)
                                + ",\"method\":" + quote(method_name) + ",\"expression\":" + quote(expression)
                                + ",\"start_line\":" + unit.getLineMap().getLineNumber(start)
                                + ",\"end_line\":" + unit.getLineMap().getLineNumber(Math.max(start, end - 1))
                                + ",\"source_sha256\":" + quote(digest(body)) + "}");
                    }

                    @Override public Void visitClass(ClassTree node, Void unused) {
                        String previous = class_name;
                        String name = node.getSimpleName().toString();
                        class_name = previous.isEmpty() ? name : previous + "." + (name.isEmpty() ? "anonymous" : name);
                        emit(node, "class", class_name, node.getExtendsClause() == null ? "" : node.getExtendsClause().toString());
                        super.visitClass(node, unused);
                        class_name = previous;
                        return null;
                    }

                    @Override public Void visitMethod(MethodTree node, Void unused) {
                        String previous = method_name;
                        method_name = node.getName() + "(" + String.join(",", node.getParameters().stream()
                                .map(parameter -> parameter.getType().toString()).toList()) + ")";
                        String declaration = node.getModifiers() + " " + (node.getReturnType() == null ? "" : node.getReturnType());
                        emit(node, "method", method_name, declaration.trim());
                        super.visitMethod(node, unused);
                        method_name = previous;
                        return null;
                    }

                    @Override public Void visitVariable(VariableTree node, Void unused) {
                        TreePath parent = getCurrentPath().getParentPath();
                        if (parent != null && parent.getLeaf() instanceof ClassTree) {
                            emit(node, "field", node.getName().toString(), node.getType() + " " + node.getModifiers()
                                    + "= " + (node.getInitializer() == null ? "" : node.getInitializer()));
                        }
                        return super.visitVariable(node, unused);
                    }

                    @Override public Void visitBlock(BlockTree node, Void unused) {
                        TreePath parent = getCurrentPath().getParentPath();
                        if (parent != null && parent.getLeaf() instanceof ClassTree) emit(node, "initializer", "", "");
                        return super.visitBlock(node, unused);
                    }

                    @Override public Void visitIf(IfTree node, Void unused) {
                        emit(node, "if", "", node.getCondition().toString());
                        return super.visitIf(node, unused);
                    }
                    @Override public Void visitCase(CaseTree node, Void unused) {
                        emit(node, "case", "", node.getLabels().toString());
                        return super.visitCase(node, unused);
                    }
                    @Override public Void visitConditionalExpression(ConditionalExpressionTree node, Void unused) {
                        emit(node, "conditional", "", node.getCondition().toString());
                        return super.visitConditionalExpression(node, unused);
                    }
                    @Override public Void visitForLoop(ForLoopTree node, Void unused) {
                        emit(node, "for", "", node.getInitializer() + "; " + node.getCondition() + "; " + node.getUpdate());
                        return super.visitForLoop(node, unused);
                    }
                    @Override public Void visitEnhancedForLoop(EnhancedForLoopTree node, Void unused) {
                        emit(node, "for_each", "", node.getVariable() + " : " + node.getExpression());
                        return super.visitEnhancedForLoop(node, unused);
                    }
                    @Override public Void visitWhileLoop(WhileLoopTree node, Void unused) {
                        emit(node, "while", "", node.getCondition().toString());
                        return super.visitWhileLoop(node, unused);
                    }
                    @Override public Void visitDoWhileLoop(DoWhileLoopTree node, Void unused) {
                        emit(node, "do_while", "", node.getCondition().toString());
                        return super.visitDoWhileLoop(node, unused);
                    }
                    @Override public Void visitCatch(CatchTree node, Void unused) {
                        emit(node, "catch", "", node.getParameter().toString());
                        return super.visitCatch(node, unused);
                    }
                    @Override public Void visitSynchronized(SynchronizedTree node, Void unused) {
                        emit(node, "synchronized", "", node.getExpression().toString());
                        return super.visitSynchronized(node, unused);
                    }
                }.scan(unit, null);
            }
        }
        for (var diagnostic : diagnostics.getDiagnostics()) {
            if (diagnostic.getKind() == Diagnostic.Kind.ERROR) throw new IllegalArgumentException(diagnostic.toString());
        }
        for (String line : output) System.out.println(line);
    }
}
