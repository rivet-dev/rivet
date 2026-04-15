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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDitto.js
var require_faDitto = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDitto.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ditto";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "22";
    var svgPathData = "M0 96l0 2.3c0 3.8 .3 7.6 1 11.4L26.5 256.5C29.7 274.7 45.5 288 64 288s34.3-13.3 37.5-31.5L127 109.6c.7-3.8 1-7.5 1-11.4l0-2.3c0-35.3-28.7-64-64-64S0 60.7 0 96zm192 0l0 2.3c0 3.8 .3 7.6 1 11.4l25.5 146.8c3.2 18.2 19 31.5 37.5 31.5s34.3-13.3 37.5-31.5L319 109.6c.7-3.8 1-7.5 1-11.4l0-2.3c0-35.3-28.7-64-64-64s-64 28.7-64 64z";
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
    exports.faDitto = exports.definition;
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

// src/_entries/faDitto.js
var import_faDitto = __toESM(require_faDitto());
var export_faDitto = import_faDitto.definition;
export {
  export_faDitto as faDitto
};
