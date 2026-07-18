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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faScarf.js
var require_faScarf = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faScarf.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "scarf";
    var width = 512;
    var height = 512;
    var aliases = [129507];
    var unicode = "f7c1";
    var svgPathData = "M256 137.4L374.6 256l9.4 9.4 11.3 11.3 112 112c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0L384 310.6 358.6 336 459.3 436.7c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0L336 358.6 310.6 384 411.3 484.7c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0l-112-112L128.4 247C107.7 226.3 96 198.1 96 168.8c0-19 4.9-37.7 14.2-54.2l41.5-73.8C165.9 15.6 192.5 0 221.4 0l69.1 0c28.9 0 55.6 15.6 69.7 40.8l41.5 73.8c9.3 16.6 14.2 35.2 14.2 54.2c0 22.8-7 44.9-19.9 63.4L278.6 114.7l15.1-15.1C283.1 97.5 270.3 96 256 96s-27.1 1.5-37.8 3.6L256 137.4zM4.7 388.7l112-112 22.6 22.6-112 112c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6zm48 48l112-112 22.6 22.6-112 112c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6zm48 48l112-112 22.6 22.6-112 112c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6z";
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
    exports.faScarf = exports.definition;
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

// src/_entries/faScarf.js
var import_faScarf = __toESM(require_faScarf());
var export_faScarf = import_faScarf.definition;
export {
  export_faScarf as faScarf
};
