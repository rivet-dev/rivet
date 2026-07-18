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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserMusic.js
var require_faUserMusic = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserMusic.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-music";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8eb";
    var svgPathData = "M224 0a128 128 0 1 1 0 256A128 128 0 1 1 224 0zM178.3 304l91.4 0c36.3 0 70.1 10.9 98.3 29.5l0 51.6c-18 2.5-34.8 9.1-48.5 19.4c-17.6 13.2-31.5 34-31.5 59.5c0 19.1 7.7 35.4 18.9 48L29.7 512C13.3 512 0 498.7 0 482.3C0 383.8 79.8 304 178.3 304zM630 164.5c6.3 4.5 10 11.8 10 19.5l0 48 0 160c0 1.2-.1 2.4-.3 3.6c.2 1.5 .3 2.9 .3 4.4c0 26.5-28.7 48-64 48s-64-21.5-64-48s28.7-48 64-48c5.5 0 10.9 .5 16 1.5l0-88.2-144 48L448 464c0 26.5-28.7 48-64 48s-64-21.5-64-48s28.7-48 64-48c5.5 0 10.9 .5 16 1.5L400 296l0-48c0-10.3 6.6-19.5 16.4-22.8l192-64c7.3-2.4 15.4-1.2 21.6 3.3z";
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
    exports.faUserMusic = exports.definition;
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

// src/_entries/faUserMusic.js
var import_faUserMusic = __toESM(require_faUserMusic());
var export_faUserMusic = import_faUserMusic.definition;
export {
  export_faUserMusic as faUserMusic
};
