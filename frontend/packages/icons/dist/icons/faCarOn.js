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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCarOn.js
var require_faCarOn = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCarOn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "car-on";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e4dd";
    var svgPathData = "M248-8c0-13.3-10.7-24-24-24S200-21.3 200-8l0 64c0 13.3 10.7 24 24 24s24-10.7 24-24l0-64zM145.3 208l157.5 0c6.7 0 12.6 4.1 15 10.4l26.1 69.6-239.6 0 26.1-69.6c2.3-6.2 8.3-10.4 15-10.4zM34 292.8l-1.3 3.4C13.2 307.1 0 328 0 352L0 480c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-32 288 0 0 32c0 17.7 14.3 32 32 32l16 0c17.7 0 32-14.3 32-32l0-128c0-24-13.2-44.9-32.8-55.9l-1.3-3.4-36.3-96.9c-11.7-31.2-41.6-51.9-74.9-51.9l-157.5 0c-33.3 0-63.2 20.7-74.9 51.9L34 292.8zM96 336a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm224 32a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM7 7C-2.3 16.4-2.3 31.6 7 41L55 89c9.4 9.4 24.6 9.4 33.9 0S98.3 64.4 89 55L41 7C31.6-2.3 16.4-2.3 7 7zM407 7L359 55c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0l48-48c9.4-9.4 9.4-24.6 0-33.9S416.4-2.3 407 7z";
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
    exports.faCarOn = exports.definition;
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

// src/_entries/faCarOn.js
var import_faCarOn = __toESM(require_faCarOn());
var export_faCarOn = import_faCarOn.definition;
export {
  export_faCarOn as faCarOn
};
