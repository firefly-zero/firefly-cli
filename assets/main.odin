package main

import "./vendor/firefly"
import "base:runtime"

@(export = true)
boot :: proc "contextless" () {
	context = runtime.default_context()
	// ...
}

@(export = true)
update :: proc "contextless" () {
	context = runtime.default_context()
	// ...
}

@(export = true)
render :: proc "contextless" () {
	context = runtime.default_context()
	firefly.clear_screen(firefly.Color.White)
	firefly.draw_triangle(
		firefly.p(60, 10),
		firefly.p(40, 40),
		firefly.p(80, 40),
		firefly.Style{firefly.Color.DarkBlue, firefly.Color.Blue, 1},
	)
}
