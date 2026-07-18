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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGaugeSimpleMin.js
var require_faGaugeSimpleMin = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGaugeSimpleMin.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "gauge-simple-min";
    var width = 512;
    var height = 512;
    var aliases = ["tachometer-slowest"];
    var unicode = "f62d";
    var svgPathData = "M0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zm320 96c0-35.3-28.7-64-64-64c-14.1 0-27.2 4.6-37.8 12.4L100.6 227.6c-11.3-7-26.1-3.5-33 7.8s-3.5 26.1 7.8 33l117.5 72.8c-.6 3.5-.9 7.1-.9 10.8c0 35.3 28.7 64 64 64s64-28.7 64-64z";
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
    exports.faGaugeSimpleMin = exports.definition;
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

// src/_entries/faTachometerSlowest.js
var import_faGaugeSimpleMin = __toESM(require_faGaugeSimpleMin());
var export_faTachometerSlowest = import_faGaugeSimpleMin.definition;
export {
  export_faTachometerSlowest as faTachometerSlowest
};
