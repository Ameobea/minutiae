/**
 * Functions for generating color schemes, transforming colors, etc.
 */

import _ from 'lodash';
import { scale } from 'chroma-js';

/**
 * Given an array of color strings, returns a function that returns scaled values along that range.  The inputs for the returned function
 * should be floating point numbers from 0 to 1, and the returned values will be hex-encoded color strings.
 * @arg scalePointsString: A comma-separated string containing a list of scale points.
 */
export const genScale = scalePointsString => {
  const scalePointsArray = _.split(_.replace(scalePointsString, /\ /g, ''), /,|,\ /g);
  return scale(scalePoints).domain(0.0, 1.0).mode('lab');
};

/**
 * Darkens a color by a somewhat arbitrary value.  The color should be a Chroma.JS-compatible color string.
 */
export const darken = chroma.darken;

/**
 * Brightens a color by a somewhat arbitrary value.  The color should be a Chroma.JS-compatible color string.
 */
export const brighten = chroma.brighten;
