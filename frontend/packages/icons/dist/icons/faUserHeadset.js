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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHeadset.js
var require_faUserHeadset = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHeadset.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-headset";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f82d";
    var svgPathData = "M224 32C135.6 32 64 103.6 64 192l0 16c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-16C32 86 118 0 224 0S416 86 416 192l0 16c0 61.9-50.1 112-112 112l-64 0-16 0-16 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l32 0c17.7 0 32 14.3 32 32l32 0c44.2 0 80-35.8 80-80l0-16c0-88.4-71.6-160-160-160zM96 192c0-70.7 57.3-128 128-128s128 57.3 128 128c0 13.9-2.2 27.3-6.3 39.8C337.4 246.3 321.8 256 304 256l-8.6 0c-11.1-19.1-31.7-32-55.4-32l-32 0c-35.3 0-64 28.7-64 64c0 1.4 0 2.7 .1 4C114.8 268.6 96 232.5 96 192zM224 352l16 0 64 0 9.6 0C387.8 352 448 412.2 448 486.4c0 14.1-11.5 25.6-25.6 25.6L25.6 512C11.5 512 0 500.5 0 486.4C0 412.2 60.2 352 134.4 352l73.6 0 16 0z";
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
    exports.faUserHeadset = exports.definition;
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

// src/_entries/faUserHeadset.js
var import_faUserHeadset = __toESM(require_faUserHeadset());
var export_faUserHeadset = import_faUserHeadset.definition;
export {
  export_faUserHeadset as faUserHeadset
};
