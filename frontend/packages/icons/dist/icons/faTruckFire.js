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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckFire.js
var require_faTruckFire = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckFire.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-fire";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e65a";
    var svgPathData = "M0 24C0 37.3 10.7 48 24 48l8 0 0 50.7C13.4 105.3 0 123.1 0 144l0 80 384 0 0-128 0-16 0-32 232 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L24 0C10.7 0 0 10.7 0 24zM0 256L0 368c0 26.5 21.5 48 48 48l16 0c0 53 43 96 96 96s96-43 96-96l128 0c0 53 43 96 96 96s96-43 96-96l16 0c26.5 0 48-21.5 48-48l0-112 0-96c0-35.3-28.7-64-64-64l-16 0c0-17.7-14.3-32-32-32s-32 14.3-32 32l-48 0-32 0 0 144 0 16-16 0L0 256zM280 48l56 0 0 48-56 0 0-48zM232 96l-48 0 0-48 48 0 0 48zM80 48l56 0 0 48L80 96l0-48zM576 160l0 96-96 0 0-96 96 0zM160 368a48 48 0 1 1 0 96 48 48 0 1 1 0-96zm272 48a48 48 0 1 1 96 0 48 48 0 1 1 -96 0z";
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
    exports.faTruckFire = exports.definition;
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

// src/_entries/faTruckFire.js
var import_faTruckFire = __toESM(require_faTruckFire());
var export_faTruckFire = import_faTruckFire.definition;
export {
  export_faTruckFire as faTruckFire
};
