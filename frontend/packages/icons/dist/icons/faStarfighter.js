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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStarfighter.js
var require_faStarfighter = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStarfighter.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "starfighter";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e037";
    var svgPathData = "M353.8 14L384 256l0 208c0 8.8-7.2 16-16 16l-96 0c-8.8 0-16-7.2-16-16l0-208L286.2 14c1-8 7.8-14 15.9-14l35.8 0c8.1 0 14.9 6 15.9 14zM288 256l0 48c0 8.8 7.2 16 16 16l32 0c8.8 0 16-7.2 16-16l0-48c0-17.7-14.3-32-32-32s-32 14.3-32 32zM48 72c13.3 0 24 10.7 24 24l0 24c13.3 0 24 10.7 24 24s-10.7 24-24 24l0 184 56 0 0-32c0-17.7 14.3-32 32-32l32 0c17.7 0 32 14.3 32 32l0 32 0 64 0 32 0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-9.6L72 444l0 4c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-280c-13.3 0-24-10.7-24-24s10.7-24 24-24l0-24c0-13.3 10.7-24 24-24zm544 0c13.3 0 24 10.7 24 24l0 24c13.3 0 24 10.7 24 24s-10.7 24-24 24l0 280c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-4-88 26.4 0 9.6c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32 0-32 0-64 0-32c0-17.7 14.3-32 32-32l32 0c17.7 0 32 14.3 32 32l0 32 56 0 0-184c-13.3 0-24-10.7-24-24s10.7-24 24-24l0-24c0-13.3 10.7-24 24-24z";
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
    exports.faStarfighter = exports.definition;
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

// src/_entries/faStarfighter.js
var import_faStarfighter = __toESM(require_faStarfighter());
var export_faStarfighter = import_faStarfighter.definition;
export {
  export_faStarfighter as faStarfighter
};
