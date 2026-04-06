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

// src/node_modules/@fortawesome/free-solid-svg-icons/faArrowPointer.js
var require_faArrowPointer = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faArrowPointer.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "arrow-pointer";
    var width = 448;
    var height = 512;
    var aliases = ["mouse-pointer"];
    var unicode = "f245";
    var svgPathData = "M77.3 2.5c8.1-4.1 17.9-3.2 25.1 2.3l320 239.9c8.3 6.2 11.6 17 8.4 26.8s-12.4 16.4-22.8 16.4l-152.3 0 88.9 177.7c7.9 15.8 1.5 35-14.3 42.9s-35 1.5-42.9-14.3l-88.9-177.7-91.3 121.8c-6.2 8.3-17 11.6-26.8 8.4S64 434.3 64 424L64 24c0-9.1 5.1-17.4 13.3-21.5z";
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
    exports.faArrowPointer = exports.definition;
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

// src/_entries/faMousePointer.js
var import_faArrowPointer = __toESM(require_faArrowPointer());
var export_faMousePointer = import_faArrowPointer.definition;
export {
  export_faMousePointer as faMousePointer
};
