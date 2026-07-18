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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBucket.js
var require_faBucket = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBucket.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bucket";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e4cf";
    var svgPathData = "M443.7 208c2.7 4.7 4.3 10.2 4.3 16 0 17.7-14.3 32-32 32l-5.1 0-22.4 213c-2.6 24.4-23.2 43-47.8 43l-233.6 0c-24.6 0-45.2-18.5-47.8-43L37.1 256 32 256c-17.7 0-32-14.3-32-32 0-5.8 1.6-11.3 4.3-16l439.4 0zM224-16c79.5 0 144 64.5 144 144l0 32-48 0 0-32c0-53-43-96-96-96s-96 43-96 96l0 32-48 0 0-32C80 48.5 144.5-16 224-16z";
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
    exports.faBucket = exports.definition;
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

// src/_entries/faBucket.js
var import_faBucket = __toESM(require_faBucket());
var export_faBucket = import_faBucket.definition;
export {
  export_faBucket as faBucket
};
