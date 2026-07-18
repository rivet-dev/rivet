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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckContainerEmpty.js
var require_faTruckContainerEmpty = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckContainerEmpty.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-container-empty";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e2b5";
    var svgPathData = "M456 144c-13.3 0-24 10.7-24 24l0 88 0 32 0 32-224 0L80 320l-48 0c-17.7 0-32 14.3-32 32l0 32 0 16c0 44.2 35.8 80 80 80c26.2 0 49.4-12.6 64-32c14.6 19.4 37.8 32 64 32c44.2 0 80-35.8 80-80c0-5.5-.6-10.8-1.6-16L432 384l17.6 0c-1 5.2-1.6 10.5-1.6 16c0 44.2 35.8 80 80 80s80-35.8 80-80c0-5.5-.6-10.8-1.6-16l1.6 0c17.7 0 32-14.3 32-32l0-64 0-16 0-10.3c0-9.2-3.2-18.2-9-25.3l-58.8-71.8c-10.6-13-26.5-20.5-43.3-20.5L456 144zM585 256l-105 0 0-64 48.8 0c2.4 0 4.7 1.1 6.2 2.9L585 256zM528 368a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM176 400a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM80 368a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faTruckContainerEmpty = exports.definition;
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

// src/_entries/faTruckContainerEmpty.js
var import_faTruckContainerEmpty = __toESM(require_faTruckContainerEmpty());
var export_faTruckContainerEmpty = import_faTruckContainerEmpty.definition;
export {
  export_faTruckContainerEmpty as faTruckContainerEmpty
};
