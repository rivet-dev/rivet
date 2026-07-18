var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPhotoFilmMusic.js
var require_faPhotoFilmMusic = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPhotoFilmMusic.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "photo-film-music";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e228";
    var svgPathData = "M192 64c0-35.3 28.7-64 64-64L576 0c35.3 0 64 28.7 64 64l0 69.6c-12.9-6.1-27.9-7.1-41.7-2.5l-98.9 33-37.8-60.5c-2.9-4.7-8.1-7.5-13.6-7.5s-10.6 2.8-13.6 7.5L388 177.9l-15.3-19.7c-3-3.9-7.7-6.2-12.6-6.2s-9.6 2.3-12.6 6.2l-56 72c-3.8 4.8-4.4 11.4-1.7 16.9s8.3 9 14.4 9l64 0 0 64-112 0c-35.3 0-64-28.7-64-64l0-192zM319.5 404.6c-13.8 10.3-25.2 25.2-29.6 43.4L64 448c-35.3 0-64-28.7-64-64L0 160c0-35.3 28.7-64 64-64l96 0 0 264c0 17.7 14.3 32 32 32l150.2 0c-8.2 3.3-15.8 7.5-22.6 12.6zM320 96a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zM56 168l0 16c0 8.8 7.2 16 16 16l16 0c8.8 0 16-7.2 16-16l0-16c0-8.8-7.2-16-16-16l-16 0c-8.8 0-16 7.2-16 16zm16 80c-8.8 0-16 7.2-16 16l0 16c0 8.8 7.2 16 16 16l16 0c8.8 0 16-7.2 16-16l0-16c0-8.8-7.2-16-16-16l-16 0zM56 360l0 16c0 8.8 7.2 16 16 16l16 0c8.8 0 16-7.2 16-16l0-16c0-8.8-7.2-16-16-16l-16 0c-8.8 0-16 7.2-16 16zM630 164.5c6.3 4.5 10 11.8 10 19.5l0 48 0 160c0 1.2-.1 2.4-.3 3.6c.2 1.5 .3 2.9 .3 4.4c0 26.5-28.7 48-64 48s-64-21.5-64-48s28.7-48 64-48c5.5 0 10.9 .5 16 1.5l0-88.2-144 48L448 464c0 26.5-28.7 48-64 48s-64-21.5-64-48s28.7-48 64-48c5.5 0 10.9 .5 16 1.5L400 296l0-48c0-10.3 6.6-19.5 16.4-22.8l192-64c7.3-2.4 15.4-1.2 21.6 3.3z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faPhotoFilmMusic = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faPhotoFilmMusic.js
var import_faPhotoFilmMusic = __toESM(require_faPhotoFilmMusic());
var export_faPhotoFilmMusic = import_faPhotoFilmMusic.definition;
export {
  export_faPhotoFilmMusic as faPhotoFilmMusic
};
