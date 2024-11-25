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
		firefly.Point{X: 60, Y: 10},
		firefly.Point{X: 40, Y: 40},
		firefly.Point{X: 80, Y: 40},
		firefly.Style{
			FillColor:   firefly.ColorDarkBlue,
			StrokeColor: firefly.ColorBlue,
			StrokeWidth: 1,
		},
	)
}
