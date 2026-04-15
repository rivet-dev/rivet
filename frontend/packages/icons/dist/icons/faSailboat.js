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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSailboat.js
var require_faSailboat = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSailboat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sailboat";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e445";
    var svgPathData = "M240 35.9l0 348.1-224 0c-8.8 0-16 7.2-16 16 0 61.9 50.1 112 112 112l288 0c61.9 0 112-50.1 112-112 0-8.8-7.2-16-16-16l-208 0 0-48 161.7 0c12.9 0 20.5-14.4 13.2-25L269.2 26.9c-8.9-13-29.2-6.7-29.2 9zM192 320l0-184.2c0-16.1-21-22.1-29.6-8.5L47.3 311.5c-6.7 10.7 1 24.5 13.6 24.5L176 336c8.8 0 16-7.2 16-16z";
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
    exports.faSailboat = exports.definition;
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

// src/_entries/faSailboat.js
var import_faSailboat = __toESM(require_faSailboat());
var export_faSailboat = import_faSailboat.definition;
export {
  export_faSailboat as faSailboat
};
