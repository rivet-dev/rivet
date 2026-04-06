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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonFalling.js
var require_faPersonFalling = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonFalling.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-falling";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e546";
    var svgPathData = "M320 32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 5.4c0 45-23.6 86.6-62.1 109.8l-4.6 2.8C131.4 184.7 96 247.1 96 314.6L96 384c0 17.7 14.3 32 32 32s32-14.3 32-32l0-69.4c0-16.7 3.3-33 9.4-48L359.2 500.2c11.1 13.7 31.3 15.8 45 4.7s15.8-31.3 4.7-45L295.2 320 400 320 438.4 371.2c10.6 14.1 30.7 17 44.8 6.4s17-30.7 6.4-44.8l-43.2-57.6C437.3 263.1 423.1 256 408 256l-89 0-62.9-75.5c40.3-36 63.9-87.9 63.9-143.1l0-5.4zM104 144a56 56 0 1 0 0-112 56 56 0 1 0 0 112z";
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
    exports.faPersonFalling = exports.definition;
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

// src/_entries/faPersonFalling.js
var import_faPersonFalling = __toESM(require_faPersonFalling());
var export_faPersonFalling = import_faPersonFalling.definition;
export {
  export_faPersonFalling as faPersonFalling
};
