# Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
# SPDX-License-Identifier: GPL-2.0-or-later

extends RefCounted

const MAX_DELAY_MS: int = 2147483647
const CONTINUOUS_SAMPLE_BATCHES: int = 10
var current_delay_ms: int = 2
var failed_time: int = 0
var restarted: bool = false
var pause_requested: bool = false

var _continuous_batches: int = 0
var _continuous_ticks: int = 0
var _continuous_usec: int = 0
var _continuous_was_busy: bool = false


func reset(delay_ms: int) -> void:
	assert(delay_ms > 0 and delay_ms <= MAX_DELAY_MS)
	current_delay_ms = delay_ms
	failed_time = 0
	restarted = false
	pause_requested = false
	clear_continuous_sample()


func observe(elapsed_ms: float, requested_delay_ms: int) -> void:
	assert(is_finite(elapsed_ms) and elapsed_ms >= 0.0)
	assert(current_delay_ms > 0 and current_delay_ms <= MAX_DELAY_MS)
	assert(requested_delay_ms > 0 and requested_delay_ms <= MAX_DELAY_MS)
	restarted = false
	if elapsed_ms > current_delay_ms * 1.5 or current_delay_ms < requested_delay_ms:
		failed_time = maxi(failed_time + 1, 0)
		if failed_time >= 2:
			failed_time = 0
			current_delay_ms = mini(int(current_delay_ms * 1.5), MAX_DELAY_MS)
			restarted = true
	elif elapsed_ms < current_delay_ms * 1.2 and current_delay_ms > requested_delay_ms:
		failed_time = mini(failed_time - 1, 0)
		if failed_time <= -10:
			current_delay_ms = maxi(requested_delay_ms, current_delay_ms - 1)
			restarted = true
	else:
		failed_time = 0
	pause_requested = current_delay_ms > 1000


func clear_continuous_sample() -> void:
	_continuous_batches = 0
	_continuous_ticks = 0
	_continuous_usec = 0
	_continuous_was_busy = false


func observe_continuous(elapsed_usec: int, completed: int, requested_delay_ms: int, busy: bool) -> void:
	assert(elapsed_usec >= 0 and completed > 0)
	assert(requested_delay_ms > 0 and requested_delay_ms <= current_delay_ms)
	if busy != _continuous_was_busy:
		clear_continuous_sample()
		_continuous_was_busy = busy
		return
	_continuous_batches += 1
	_continuous_ticks += completed
	_continuous_usec += elapsed_usec
	if _continuous_batches < CONTINUOUS_SAMPLE_BATCHES:
		return

	# Estimate capacity from busy completion spacing and reserve 1 ms to drain old debt
	if busy:
		var service_delay_ms: int = ceili(minf(MAX_DELAY_MS, 1.0 + float(_continuous_usec) / (1000.0 * _continuous_ticks)))
		current_delay_ms = maxi(requested_delay_ms, service_delay_ms)
	else:
		current_delay_ms = maxi(requested_delay_ms, current_delay_ms - 1)
	clear_continuous_sample()
	_continuous_was_busy = busy
