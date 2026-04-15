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

// src/node_modules/@fortawesome/free-solid-svg-icons/faIgloo.js
var require_faIgloo = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faIgloo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "igloo";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f7ae";
    var svgPathData = "M320 33.8l0 126.2-271.5 0c51.7-77.2 139.6-128 239.5-128 10.8 0 21.5 .6 32 1.8zM368 160l0-116.7c66 19 122.2 61 159.5 116.7L368 160zM22.6 208l73.4 0 0 112-96 0c0-39.7 8-77.6 22.6-112zM176 320l-32 0 0-112 288 0 0 112-32 0 0 48 176 0 0 64c0 26.5-21.5 48-48 48L48 480c-26.5 0-48-21.5-48-48l0-64 176 0 0-48zm304 0l0-112 73.4 0C568 242.4 576 280.3 576 320l-96 0zM288 288c-35.3 0-64 28.7-64 64l0 80 128 0 0-80c0-35.3-28.7-64-64-64z";
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
    exports.faIgloo = exports.definition;
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

// src/_entries/faIgloo.js
var import_faIgloo = __toESM(require_faIgloo());
var export_faIgloo = import_faIgloo.definition;
export {
  export_faIgloo as faIgloo
};
