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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleQ.js
var require_faCircleQ = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleQ.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-q";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e11e";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM176 256c0 44.2 35.8 80 80 80c13.4 0 25.9-3.3 37-9.1l-30-30c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l30 30c5.8-11.1 9.1-23.7 9.1-37c0-44.2-35.8-80-80-80s-80 35.8-80 80zm208 0c0 26.7-8.2 51.4-22.1 71.9L377 343c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0l-15.1-15.1c-20.5 14-45.3 22.1-71.9 22.1c-70.7 0-128-57.3-128-128s57.3-128 128-128s128 57.3 128 128z";
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
    exports.faCircleQ = exports.definition;
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

// src/_entries/faCircleQ.js
var import_faCircleQ = __toESM(require_faCircleQ());
var export_faCircleQ = import_faCircleQ.definition;
export {
  export_faCircleQ as faCircleQ
};
