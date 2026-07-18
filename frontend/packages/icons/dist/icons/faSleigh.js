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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSleigh.js
var require_faSleigh = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSleigh.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sleigh";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f7cc";
    var svgPathData = "M64 32C46.3 32 32 46.3 32 64S46.3 96 64 96l0 160c0 41.8 26.7 77.4 64 90.5l0 69.5-72 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l504 0c44.2 0 80-35.8 80-80l0-8c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 8c0 17.7-14.3 32-32 32l-80 0 0-64c53 0 96-43 96-96l0-96c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0c-17.7 0-32 14.3-32 32l0 32c0 35.3-28.7 64-64 64l-48.9 0c-48.5 0-92.8-27.4-114.5-70.8l-25.2-50.5C237.7 59.4 193.4 32 144.9 32L64 32zM432 416l-256 0 0-64 256 0 0 64z";
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
    exports.faSleigh = exports.definition;
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

// src/_entries/faSleigh.js
var import_faSleigh = __toESM(require_faSleigh());
var export_faSleigh = import_faSleigh.definition;
export {
  export_faSleigh as faSleigh
};
