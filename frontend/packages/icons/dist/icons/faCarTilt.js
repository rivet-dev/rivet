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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCarTilt.js
var require_faCarTilt = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCarTilt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "car-tilt";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f5e5";
    var svgPathData = "M145.1 120.6L292.4 81.2c12.7-3.4 26.3 1.3 34.1 12l37.8 51.6L114.6 211.6l7-63.6c1.4-13.1 10.8-24 23.5-27.4zM58 141.1L47.8 234.2C27.4 249.4 17.2 276 24.2 302.1L57.3 425.8l6 22.2L48 448c-26.5 0-48 21.5-48 48l0 16 640 0 0-16c0-26.5-21.5-48-48-48l-429.4 0c-.1-.2-.1-.5-.2-.7L150 400.9l278.2-74.5 12.4 46.4c4.6 17.1 22.1 27.2 39.2 22.6l30.9-8.3c17.1-4.6 27.2-22.1 22.6-39.2L521 301.5 487.8 177.9c-7-26.1-29.1-44.1-54.4-47L378.1 55.3c-23.4-31.9-64-46.2-102.3-36L128.5 58.8C90.3 69.1 62.3 101.7 58 141.1zm58.9 136.2a32 32 0 1 1 16.6 61.8 32 32 0 1 1 -16.6-61.8zm255.6-35.3a32 32 0 1 1 61.8-16.6 32 32 0 1 1 -61.8 16.6z";
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
    exports.faCarTilt = exports.definition;
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

// src/_entries/faCarTilt.js
var import_faCarTilt = __toESM(require_faCarTilt());
var export_faCarTilt = import_faCarTilt.definition;
export {
  export_faCarTilt as faCarTilt
};
