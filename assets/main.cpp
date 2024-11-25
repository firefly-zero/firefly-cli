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
    DrawTriangle(
        {60, 10}, {40, 40}, {80, 40},
        {
            .fill_color = LIGHT_GRAY,
            .stroke_color = DARK_BLUE,
            .stroke_width = 1,
        });
}
