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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTemperatureArrowDown.js
var require_faTemperatureArrowDown = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTemperatureArrowDown.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "temperature-arrow-down";
    var width = 512;
    var height = 512;
    var aliases = ["temperature-down"];
    var unicode = "e03f";
    var svgPathData = "M64.5 96c0-53 43-96 96-96s96 43 96 96l0 164.7c29.5 26.4 48 64.7 48 107.3 0 79.5-64.5 144-144 144s-144-64.5-144-144c0-42.6 18.5-81 48-107.3L64.5 96zm96 336c35.3 0 64-28.7 64-64 0-26.9-16.5-49.9-40-59.3l0-28.7c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 28.7c-23.5 9.5-40 32.5-40 59.3 0 35.3 28.7 64 64 64zM439.1 310.6c-6 6-14.1 9.4-22.6 9.4s-16.6-3.4-22.6-9.4l-64-64c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0l9.4 9.4 0-178.7c0-17.7 14.3-32 32-32s32 14.3 32 32l0 178.7 9.4-9.4c12.5-12.5 32.8-12.5 45.3 0s12.5 32.8 0 45.3l-64 64z";
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
    exports.faTemperatureArrowDown = exports.definition;
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

// src/_entries/faTemperatureArrowDown.js
var import_faTemperatureArrowDown = __toESM(require_faTemperatureArrowDown());
var export_faTemperatureArrowDown = import_faTemperatureArrowDown.definition;
export {
  export_faTemperatureArrowDown as faTemperatureArrowDown
};
