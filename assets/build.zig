const std = @import("std");

pub fn build(b: *std.Build) void {
    const target_query: std.Target.Query = .{
        .cpu_arch = .wasm32,
        .os_tag = .freestanding,
    };
    const target = b.resolveTargetQuery(target_query);
    const optimize: std.builtin.OptimizeMode = .ReleaseSmall;
    const exe = b.addExecutable(.{
        .name = "main",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
            .strip = true,
        }),
    });
    exe.entry = .disabled;
    exe.rdynamic = true;
    exe.stack_size = 4096;
    const firefly_package = b.dependency("firefly", .{});
    const firefly_module = firefly_package.module("firefly");
    exe.root_module.addImport("firefly", firefly_module);
    b.installArtifact(exe);
}
