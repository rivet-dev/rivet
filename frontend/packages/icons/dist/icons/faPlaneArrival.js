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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPlaneArrival.js
var require_faPlaneArrival = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPlaneArrival.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "plane-arrival";
    var width = 576;
    var height = 512;
    var aliases = [128748];
    var unicode = "f5af";
    var svgPathData = "M386.6 193.1L265.9 3.7c-4.8-7.6-12.6-12.8-21.4-14.3l-43.1-7.6c-10.8-1.9-20.2 7.3-18.6 18.1l24 161.6-105-18.5-33.8-61.8C64.5 74.7 58.2 70.2 51.1 69L34 66c-9.8-1.7-18.8 5.9-18.8 15.8l.6 106.3c.2 30.9 22.4 57.3 52.9 62.7l13.5 2.4 0 0 417.6 73.6c30.5 5.4 59.5-15 64.9-45.4s-15-59.5-45.4-64.9L386.6 193.1zM224 384a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm131.2-15.3a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zM32 448c-17.7 0-32 14.3-32 32s14.3 32 32 32l512 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L32 448z";
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
    exports.faPlaneArrival = exports.definition;
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

// src/_entries/faPlaneArrival.js
var import_faPlaneArrival = __toESM(require_faPlaneArrival());
var export_faPlaneArrival = import_faPlaneArrival.definition;
export {
  export_faPlaneArrival as faPlaneArrival
};
