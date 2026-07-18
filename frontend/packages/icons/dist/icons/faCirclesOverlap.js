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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCirclesOverlap.js
var require_faCirclesOverlap = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCirclesOverlap.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circles-overlap";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e600";
    var svgPathData = "M192 288c0 34.2 7.7 66.6 21.4 95.7C130 378.2 64 308.8 64 224c0-88.4 71.6-160 160-160c32.2 0 62.2 9.5 87.3 25.9C240.4 127.5 192 202.1 192 288zm32 160c11.2 0 22.1-.8 32.8-2.4c40.6 41 96.9 66.4 159.2 66.4c123.7 0 224-100.3 224-224S539.7 64 416 64c-11.2 0-22.1 .8-32.8 2.4C342.6 25.4 286.3 0 224 0C100.3 0 0 100.3 0 224S100.3 448 224 448zm192 0c-32.2 0-62.2-9.5-87.3-25.9C399.6 384.5 448 309.9 448 224c0-34.2-7.7-66.6-21.4-95.7C510 133.8 576 203.2 576 288c0 88.4-71.6 160-160 160z";
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
    exports.faCirclesOverlap = exports.definition;
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

// src/_entries/faCirclesOverlap.js
var import_faCirclesOverlap = __toESM(require_faCirclesOverlap());
var export_faCirclesOverlap = import_faCirclesOverlap.definition;
export {
  export_faCirclesOverlap as faCirclesOverlap
};
