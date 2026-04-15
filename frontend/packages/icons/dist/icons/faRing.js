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

// src/node_modules/@fortawesome/free-solid-svg-icons/faRing.js
var require_faRing = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faRing.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ring";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f70b";
    var svgPathData = "M64 208c0 7.8 4.4 18.7 17.1 30.3 45.4-24.2 107.7-38.3 174.9-38.3s129.5 14.1 174.9 38.3c12.8-11.6 17.1-22.5 17.1-30.3 0-12.3-10.8-32-47.9-50.6-35.2-17.6-86.1-29.4-144.1-29.4s-108.9 11.8-144.1 29.4C74.8 176 64 195.7 64 208zm192 40c-47 0-89.3 7.6-122.9 19.7 33.2 12.4 75.7 20.3 122.9 20.3s89.7-7.8 122.9-20.3C345.3 255.6 303 248 256 248zM0 208C0 158.4 39.4 122.2 83.3 100.2 129.1 77.3 190.3 64 256 64s126.9 13.3 172.7 36.2c43.9 22 83.3 58.2 83.3 107.8l0 96c0 49.6-39.4 85.8-83.3 107.8-45.9 22.9-107 36.2-172.7 36.2S129.1 434.7 83.3 411.8C39.4 389.8 0 353.6 0 304l0-96z";
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
    exports.faRing = exports.definition;
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

// src/_entries/faRing.js
var import_faRing = __toESM(require_faRing());
var export_faRing = import_faRing.definition;
export {
  export_faRing as faRing
};
