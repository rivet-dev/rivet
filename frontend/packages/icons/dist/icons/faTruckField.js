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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTruckField.js
var require_faTruckField = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTruckField.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-field";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e58d";
    var svgPathData = "M32 96c0-35.3 28.7-64 64-64l224 0c23.7 0 44.4 12.9 55.4 32l40.4 0c32.4 0 61.6 19.6 74 49.5l43.8 106.3 1.7 4.2 24.7 0c26.5 0 48 21.5 48 48l0 64c0 26.5-21.5 48-48 48l-.4 0c.2 2.6 .4 5.3 .4 8 0 48.6-39.4 88-88 88s-88-39.4-88-88c0-2.7 .1-5.4 .4-8l-144.7 0c.2 2.6 .4 5.3 .4 8 0 48.6-39.4 88-88 88s-88-39.4-88-88c0-3.6 .2-7.1 .6-10.5-19-6.4-32.6-24.4-32.6-45.5l0-48c-17.7 0-32-14.3-32-32l0-96c0-17.7 14.3-32 32-32l0-32zm352 32l0 96 82.1 0-35.4-86.1c-2.5-6-8.3-9.9-14.8-9.9L384 128zM192 392a40 40 0 1 0 -80 0 40 40 0 1 0 80 0zm280 40a40 40 0 1 0 0-80 40 40 0 1 0 0 80z";
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
    exports.faTruckField = exports.definition;
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

// src/_entries/faTruckField.js
var import_faTruckField = __toESM(require_faTruckField());
var export_faTruckField = import_faTruckField.definition;
export {
  export_faTruckField as faTruckField
};
