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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSeatAirline.js
var require_faSeatAirline = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSeatAirline.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "seat-airline";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e244";
    var svgPathData = "M64 32C64 14.3 49.7 0 32 0S0 14.3 0 32L0 143.6c0 15.2 3.1 30.2 9.1 44.1L82 357.8c15.1 35.3 49.8 58.2 88.2 58.2l45.8 0 0 48-96 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l240 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-96 0 0-48 96 0c15.1 0 29.3-7.1 38.4-19.2l43.2-57.6c7.3-9.7 8.4-22.7 3-33.5S428.1 288 416 288l-294.3 0L108 256l212 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L80.5 192 67.9 162.5c-2.6-6-3.9-12.4-3.9-18.9L64 32z";
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
    exports.faSeatAirline = exports.definition;
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

// src/_entries/faSeatAirline.js
var import_faSeatAirline = __toESM(require_faSeatAirline());
var export_faSeatAirline = import_faSeatAirline.definition;
export {
  export_faSeatAirline as faSeatAirline
};
