package main

import "github.com/firefly-zero/firefly-go/firefly"

func init() {
	firefly.Boot = boot
	firefly.Update = update
	firefly.Render = render
}

func boot() {
	// ...
}

func update() {
	// ...
}

func render() {
	firefly.ClearScreen(firefly.ColorWhite)
	firefly.DrawTriangle(
		firefly.P(60, 10),
		firefly.P(40, 40),
		firefly.P(80, 40),
		firefly.Style{
			FillColor:   firefly.ColorDarkBlue,
			StrokeColor: firefly.ColorBlue,
			StrokeWidth: 1,
		},
	)
}
