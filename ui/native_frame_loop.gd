class_name BgrNativeFrameLoop
extends SceneTree

var _native_sampler: Callable = Callable()


func register_native_sampler(callback: Callable) -> bool:
	if not callback.is_valid() or _native_sampler.is_valid():
		return false
	_native_sampler = callback
	return true


func clear_native_sampler(callback: Callable) -> void:
	if _native_sampler == callback:
		_native_sampler = Callable()


func _process(_delta: float) -> bool:
	if _native_sampler.is_valid():
		_native_sampler.call()
	return false
