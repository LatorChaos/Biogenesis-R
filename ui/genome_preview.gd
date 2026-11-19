extends Control

signal copy_requested

var copy_enabled: bool = false
var fit_visible_content: bool = false
var fill_available_area: bool = false
var _raster: ImageTexture
var _raster_pixels: PackedByteArray
var _raster_fitted: bool = false
var _popup: PopupMenu
var _portrait_source: Rect2i


func _ready() -> void:
	texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	mouse_filter = Control.MOUSE_FILTER_STOP if copy_enabled else Control.MOUSE_FILTER_IGNORE
	_popup = PopupMenu.new()
	_popup.name = "PreviewCopyPopup"
	_popup.add_item("Copy genotype", 0)
	_popup.id_pressed.connect(func(_id: int) -> void: copy_requested.emit())
	add_child(_popup)


func set_copy_caption(caption: String) -> void:
	assert(_popup != null, "Initialize the preview before localizing its menu")
	_popup.set_item_text(0, caption)


func has_open_popup() -> bool:
	return _popup != null and _popup.visible


func _gui_input(event: InputEvent) -> void:
	if copy_enabled and event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_RIGHT and not event.pressed:
		var screen_position: Vector2 = event.global_position
		if not get_viewport().gui_embed_subwindows:
			screen_position += Vector2(get_window().position)
		_popup.position = Vector2i(screen_position)
		_popup.popup()
		accept_event()


static func valid_raster(image: Dictionary, dimensions: Vector2i = Vector2i(100, 100)) -> bool:
	if image.is_empty():
		return true
	var maximum_bytes: int = 64 * 1024 * 1024
	if dimensions.x <= 0 or dimensions.y <= 0 or dimensions.x > maximum_bytes or dimensions.y > maximum_bytes:
		return false
	var bytes: int = dimensions.x * dimensions.y * 3
	return (
		bytes <= maximum_bytes and image.get("width") is int and image["width"] == dimensions.x
		and image.get("height") is int and image["height"] == dimensions.y
		and image.get("rgb") is PackedByteArray and image["rgb"].size() == bytes
	)


func set_raster(image: Dictionary, dimensions: Vector2i = Vector2i(100, 100)) -> bool:
	if not valid_raster(image, dimensions):
		return false
	if image.is_empty():
		_raster = null
		_raster_pixels = PackedByteArray()
		_portrait_source = Rect2i()
		queue_redraw()
		return true
	var pixels: PackedByteArray = image["rgb"]
	if _raster != null and _raster.get_size() == Vector2(dimensions) and pixels == _raster_pixels and fit_visible_content == _raster_fitted:
		return true
	var content: Image = Image.create_from_data(dimensions.x, dimensions.y, false, Image.FORMAT_RGB8, pixels)
	if content == null or content.is_empty():
		return false
	if _raster == null or _raster.get_width() != dimensions.x or _raster.get_height() != dimensions.y:
		_raster = ImageTexture.create_from_image(content)
	else:
		_raster.update(content)
	_portrait_source = _content_bounds(pixels, dimensions) if fit_visible_content else Rect2i(Vector2i.ZERO, dimensions)
	_raster_pixels = pixels.duplicate()
	_raster_fitted = fit_visible_content
	queue_redraw()
	return _raster != null


static func _content_bounds(pixels: PackedByteArray, dimensions: Vector2i) -> Rect2i:
	var minimum: Vector2i = dimensions
	var maximum: Vector2i = Vector2i(-1, -1)
	for y: int in range(dimensions.y):
		for x: int in range(dimensions.x):
			var offset: int = (y * dimensions.x + x) * 3
			if pixels[offset] != 0 or pixels[offset + 1] != 0 or pixels[offset + 2] != 0:
				minimum.x = mini(minimum.x, x)
				minimum.y = mini(minimum.y, y)
				maximum.x = maxi(maximum.x, x)
				maximum.y = maxi(maximum.y, y)
	if maximum.x < 0:
		return Rect2i(Vector2i.ZERO, dimensions)
	minimum = Vector2i(maxi(0, minimum.x - 2), maxi(0, minimum.y - 2))
	maximum = Vector2i(mini(dimensions.x, maximum.x + 3), mini(dimensions.y, maximum.y + 3))
	return Rect2i(minimum, maximum - minimum)


func _draw() -> void:
	draw_rect(Rect2(Vector2.ZERO, size), Color.BLACK)
	if _raster != null:
		if fit_visible_content:
			draw_texture_rect_region(_raster, _fit_target(), Rect2(_portrait_source))
		else:
			draw_texture(_raster, Vector2.ZERO)


func _fit_target() -> Rect2:
	var source_size: Vector2 = Vector2(_portrait_source.size)
	var available: Vector2 = (size - Vector2(4.0, 4.0)).max(Vector2.ZERO) if fill_available_area else size
	var factor: float = minf(available.x / source_size.x, available.y / source_size.y)
	if not fill_available_area:
		factor = minf(6.0, factor)
	var target_size: Vector2 = source_size * factor
	return Rect2((size - target_size) * 0.5, target_size)
