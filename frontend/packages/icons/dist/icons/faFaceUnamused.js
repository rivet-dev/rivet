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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceUnamused.js
var require_faFaceUnamused = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceUnamused.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-unamused";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e39f";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zm60.1-117.3c-11.7-13-31.6-26.7-60.1-26.7s-48.4 13.7-60.1 26.7c-5.9 6.6-16 7.1-22.6 1.2s-7.1-16-1.2-22.6C188.2 355.4 216.3 336 256 336s67.8 19.4 83.9 37.3c5.9 6.6 5.4 16.7-1.2 22.6s-16.7 5.4-22.6-1.2zM112 192l64 0c17.7 0 32 14.3 32 32s-14.3 32-32 32s-32-14.3-32-32l-32 0c-8.8 0-16-7.2-16-16s7.2-16 16-16zm176 16c0-8.8 7.2-16 16-16l64 0c17.7 0 32 14.3 32 32s-14.3 32-32 32s-32-14.3-32-32l-32 0c-8.8 0-16-7.2-16-16z";
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
    exports.faFaceUnamused = exports.definition;
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

// src/_entries/faFaceUnamused.js
var import_faFaceUnamused = __toESM(require_faFaceUnamused());
var export_faFaceUnamused = import_faFaceUnamused.definition;
export {
  export_faFaceUnamused as faFaceUnamused
};
