Fork of Eric Lengyel's Slug reference repository.


I got excited and vibe coded a wgpu integration.


I'm told this is basically the same as wgpu-font-renderer (sort of)

I'm also told that we are doing a bunch of shit that should be automatically handled by rustybuzz, everything except checking magic numbers ofr mislabelled ttc files.

Anyway there is also vello which sounds pretty good

## What are we trying to actually build here: a few options
### Owned Rust text renderer for games
* With precise control over size and positioning
* Stuff like colour handling, effects, outlines
* Maybe some cheeky like cursed effects, text is moving, etc

Multiple approaches eg traditional pixel raster webgpu could use render passes, shaders, jump flooding for outlines, warping, pretty good control. Pixel perfect spacing, outlines and control.

Slug could be pretty dam clean and has a lot of control

Vello probably heavier but can do full canvas stuff

Muh image inclusion etc - Vello can do SVGs as well and stuff. Like im used to on web emoji prototypes and etc (like skia / chrome)


### Canvas Renderer thing ??
Pretty hectic but its just a vague option with vello, can draw boxes and stuff, again web tech is nice. Servo renderer include in the game lol? Its like i wonder if Dioxus native uses that. Anyway these get a bit crazy

Its like how good the game ui in HTML is working in web