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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUtilityPole.js
var require_faUtilityPole = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUtilityPole.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "utility-pole";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2c3";
    var svgPathData = "M256 0c17.7 0 32 14.3 32 32l0 32 96 0 0-8c0-13.3 10.7-24 24-24s24 10.7 24 24l0 8 32 0 0-8c0-13.3 10.7-24 24-24s24 10.7 24 24l0 40c0 17.7-14.3 32-32 32l-51.2 0L288 221.9l0 66.1 0 192c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-192 0-66.1L83.2 128 32 128C14.3 128 0 113.7 0 96L0 56C0 42.7 10.7 32 24 32s24 10.7 24 24l0 8 32 0 0-8c0-13.3 10.7-24 24-24s24 10.7 24 24l0 8 96 0 0-32c0-17.7 14.3-32 32-32zM140.8 128L224 183.4l0-55.4-83.2 0zM288 183.4L371.2 128 288 128l0 55.4z";
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
    exports.faUtilityPole = exports.definition;
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

// src/_entries/faUtilityPole.js
var import_faUtilityPole = __toESM(require_faUtilityPole());
var export_faUtilityPole = import_faUtilityPole.definition;
export {
  export_faUtilityPole as faUtilityPole
};
