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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faToiletPaperXmark.js
var require_faToiletPaperXmark = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faToiletPaperXmark.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "toilet-paper-xmark";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5b3";
    var svgPathData = "M96 0L348.2 0C336.6 12.2 327 26.9 319.5 41.9C299.5 82 288 135.1 288 192s11.5 110 31.5 150.1c7.5 15 17.1 29.7 28.7 41.9L96 384C43 384 0 298 0 192S43 0 96 0zM320 192c0-83.6 26.7-154.7 64-181.1C394 3.8 404.8 0 416 0c53 0 96 86 96 192c0 35-4.7 67.8-12.8 96c-16.6 57.4-47.6 96-83.2 96c-11.2 0-22-3.8-32-10.9c-37.3-26.4-64-97.5-64-181.1zm64 0c0 35.3 14.3 64 32 64s32-28.7 32-64s-14.3-64-32-64s-32 28.7-32 64zM160 416l256 0c44.8 0 76.9-35.2 96-72.9L512 464c0 26.5-21.5 48-48 48l-256 0c-26.5 0-48-21.5-48-48l0-48zm51.3-283.3c-6.2-6.2-16.4-6.2-22.6 0L152 169.4l-36.7-36.7c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6L129.4 192 92.7 228.7c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0L152 214.6l36.7 36.7c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6L174.6 192l36.7-36.7c6.2-6.2 6.2-16.4 0-22.6z";
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
    exports.faToiletPaperXmark = exports.definition;
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

// src/_entries/faToiletPaperXmark.js
var import_faToiletPaperXmark = __toESM(require_faToiletPaperXmark());
var export_faToiletPaperXmark = import_faToiletPaperXmark.definition;
export {
  export_faToiletPaperXmark as faToiletPaperXmark
};
