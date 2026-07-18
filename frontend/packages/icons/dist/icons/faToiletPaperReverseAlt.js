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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faToiletPaperBlankUnder.js
var require_faToiletPaperBlankUnder = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faToiletPaperBlankUnder.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "toilet-paper-blank-under";
    var width = 512;
    var height = 512;
    var aliases = ["toilet-paper-reverse-alt"];
    var unicode = "e29f";
    var svgPathData = "M348.2 0L96 0C43 0 0 86 0 192S43 384 96 384l252.2 0c-11.6-12.2-21.1-26.9-28.7-41.9C299.5 302 288 248.9 288 192s11.5-110 31.5-150.1c7.5-15 17.1-29.7 28.7-41.9zM384 10.9c-37.3 26.4-64 97.5-64 181.1s26.7 154.7 64 181.1c10 7.1 20.8 10.9 32 10.9c35.5 0 66.6-38.6 83.2-96c8.2-28.2 12.8-61 12.8-96C512 86 469 0 416 0c-11.2 0-22 3.8-32 10.9zM416 256c-17.7 0-32-28.7-32-64s14.3-64 32-64s32 28.7 32 64s-14.3 64-32 64zM160 464c0 26.5 21.5 48 48 48l256 0c26.5 0 48-21.5 48-48l0-120.9c-19.1 37.8-51.2 72.9-96 72.9l-256 0 0 48z";
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
    exports.faToiletPaperBlankUnder = exports.definition;
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

// src/_entries/faToiletPaperReverseAlt.js
var import_faToiletPaperBlankUnder = __toESM(require_faToiletPaperBlankUnder());
var export_faToiletPaperReverseAlt = import_faToiletPaperBlankUnder.definition;
export {
  export_faToiletPaperReverseAlt as faToiletPaperReverseAlt
};
