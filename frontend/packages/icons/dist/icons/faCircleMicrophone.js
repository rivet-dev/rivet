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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleMicrophone.js
var require_faCircleMicrophone = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleMicrophone.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-microphone";
    var width = 512;
    var height = 512;
    var aliases = ["microphone-circle"];
    var unicode = "e116";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM208 144c0-26.5 21.5-48 48-48s48 21.5 48 48l0 112c0 26.5-21.5 48-48 48s-48-21.5-48-48l0-112zm-64 80c8.8 0 16 7.2 16 16l0 16c0 53 43 96 96 96s96-43 96-96l0-16c0-8.8 7.2-16 16-16s16 7.2 16 16l0 16c0 65.3-48.9 119.1-112 127l0 17c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-17c-63.1-7.9-112-61.7-112-127l0-16c0-8.8 7.2-16 16-16z";
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
    exports.faCircleMicrophone = exports.definition;
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

// src/_entries/faCircleMicrophone.js
var import_faCircleMicrophone = __toESM(require_faCircleMicrophone());
var export_faCircleMicrophone = import_faCircleMicrophone.definition;
export {
  export_faCircleMicrophone as faCircleMicrophone
};
