import * as ff from "firefly-as/assembly";

export function render(): void {
  ff.clearScreen(ff.Color.White);
  ff.drawTriangle(
    ff.Point.new(50, 20),
    ff.Point.new(30, 50),
    ff.Point.new(70, 50),
    ff.Style.new(ff.Color.LightBlue, ff.Color.DarkBlue, 1)
  );
}
