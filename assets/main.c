#include "./vendor/firefly/firefly.c"

BOOT void boot()
{
    // ...
}

UPDATE void update()
{
    // ...
}

RENDER void render()
{
    ClearScreen(WHITE);
    Point p1 = {60, 10};
    Point p2 = {40, 40};
    Point p3 = {80, 40};
    Style s = {
        .fill_color = LIGHT_GRAY,
        .stroke_color = DARK_BLUE,
        .stroke_width = 1};
    DrawTriangle(p1, p2, p3, s);
}
