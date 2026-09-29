package main

import "./vendor/firefly"
import "base:runtime"

default_context: runtime.Context

@(export = true)
boot :: proc "contextless" () {
	default_context = runtime.default_context()
	context = default_context
	default_context.allocator = runtime.default_wasm_allocator()
	// ...
}

@(export = true)
update :: proc "contextless" () {
	context = default_context
	// ...
}

@(export = true)
render :: proc "contextless" () {
	context = default_context
	firefly.clear_screen(firefly.Color.White)
	firefly.draw_triangle(
		firefly.p(60, 10),
		firefly.p(40, 40),
		firefly.p(80, 40),
		firefly.Style{firefly.Color.DarkBlue, firefly.Color.Blue, 1},
	)
}
