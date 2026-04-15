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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPaintbrushFine.js
var require_faPaintbrushFine = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPaintbrushFine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "paintbrush-fine";
    var width = 576;
    var height = 512;
    var aliases = ["paint-brush-alt", "paint-brush-fine", "paintbrush-alt"];
    var unicode = "f5a9";
    var svgPathData = "M559.6 16.4c21.9 21.9 21.9 57.3 0 79.2L306.9 348.3l-79.2-79.2L480.4 16.4c21.9-21.9 57.3-21.9 79.2 0zM205.1 291.7l79.2 79.2-.1 .1c2.5 9.3 3.8 19 3.8 29c0 61.9-50.1 112-112 112L32 512c-17.7 0-32-14.3-32-32s14.3-32 32-32l4.8 0c17.6 0 29.6-18.9 27.8-36.4c-.4-3.8-.6-7.7-.6-11.6c0-61.9 50.1-112 112-112c10 0 19.8 1.3 29 3.8l.1-.1z";
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
    exports.faPaintbrushFine = exports.definition;
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

// src/_entries/faPaintBrushAlt.js
var import_faPaintbrushFine = __toESM(require_faPaintbrushFine());
var export_faPaintBrushAlt = import_faPaintbrushFine.definition;
export {
  export_faPaintBrushAlt as faPaintBrushAlt
};
