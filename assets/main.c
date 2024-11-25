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

    Point p1;
    p1.x = 60;
    p1.y = 10;

    Point p2;
    p2.x = 40;
    p2.y = 40;

    Point p3;
    p3.x = 80;
    p3.y = 40;

    Style s;
    s.fill_color = LIGHT_GRAY;
    s.stroke_color = DARK_BLUE;
    s.stroke_width = 1;

    DrawTriangle(p1, p2, p3, s);
}
