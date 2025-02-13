const ff = @import("firefly");

pub export fn render() void {
    ff.drawTriangle(
        ff.Point{ .x = 60, .y = 10 },
        ff.Point{ .x = 40, .y = 40 },
        ff.Point{ .x = 80, .y = 40 },
        ff.Style{
            .fill_color = ff.Color.light_gray,
            .stroke_color = ff.Color.dark_blue,
            .stroke_width = 1,
        },
    );
}
