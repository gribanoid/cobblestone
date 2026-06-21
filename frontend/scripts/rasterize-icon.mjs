#!/usr/bin/env node
/** Rasterize an SVG to PNG at exact pixel size (crisp pixel-art upscaling). */
import { readFileSync, writeFileSync } from 'node:fs'
import { Resvg } from '@resvg/resvg-js'

const [svgPath, outPath, sizeArg = '1024'] = process.argv.slice(2)
if (!svgPath || !outPath) {
  console.error('Usage: rasterize-icon.mjs <in.svg> <out.png> [size]')
  process.exit(1)
}

const size = Number.parseInt(sizeArg, 10)
const svg = readFileSync(svgPath)
const resvg = new Resvg(svg, {
  fitTo: { mode: 'width', value: size },
})
writeFileSync(outPath, resvg.render().asPng())
