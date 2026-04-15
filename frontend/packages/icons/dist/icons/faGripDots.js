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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGripDots.js
var require_faGripDots = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGripDots.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "grip-dots";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e410";
    var svgPathData = "M336 176a48 48 0 1 0 96 0 48 48 0 1 0 -96 0zm-160 0a48 48 0 1 0 96 0 48 48 0 1 0 -96 0zM64 224a48 48 0 1 0 0-96 48 48 0 1 0 0 96zM336 336a48 48 0 1 0 96 0 48 48 0 1 0 -96 0zM224 384a48 48 0 1 0 0-96 48 48 0 1 0 0 96zM16 336a48 48 0 1 0 96 0 48 48 0 1 0 -96 0z";
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
    exports.faGripDots = exports.definition;
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

// src/_entries/faGripDots.js
var import_faGripDots = __toESM(require_faGripDots());
var export_faGripDots = import_faGripDots.definition;
export {
  export_faGripDots as faGripDots
};
