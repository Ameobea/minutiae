# Liquid Gold

I envision a world filled with procedurally generated ore or rocks.  There will be an ore vein of sorts running through the rocks.  A gold fluid begins to consume the ore, replacing it with additional gold fluid.  The gold fluid should be shimmering and iridescent.

## World Generation

I plan on using composed noise functions to create the world.  I plan on expanding the current `noise_asmjs` implementation to allow for GUI-based noise function composition in the browser.

Right now, I see billow noise as a strong possibility for the ore.  I'd love to see That style of branch-y, tendril-y noise that looks almost organic.  I envision using one noise function for the rock and another one for the ore, keeping two separate states for both the rock and the ore.

## Fluid Implementation

I plan on creating an "intensity" value for the fluid that determines how quickly it spreads and what pockets of ore it's able to reach.  This intensity should change based on the rate that the fluid consumes ore, increasing as the rate of ore consumption decreases and vice-versa.  This should keep the rate of ore consumption constant and allow for it to be tapered off as ore reserves are consumed.
