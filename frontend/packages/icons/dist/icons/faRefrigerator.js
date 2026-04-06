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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRefrigerator.js
var require_faRefrigerator = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRefrigerator.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "refrigerator";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e026";
    var svgPathData = "M96 0C43 0 0 43 0 96l0 64 288 0 0-48c0-8.8 7.2-16 16-16s16 7.2 16 16l0 48 64 0 0-64c0-53-43-96-96-96L96 0zM384 192l-64 0 0 176c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-176L0 192 0 448c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-256z";
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
    exports.faRefrigerator = exports.definition;
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

// src/_entries/faRefrigerator.js
var import_faRefrigerator = __toESM(require_faRefrigerator());
var export_faRefrigerator = import_faRefrigerator.definition;
export {
  export_faRefrigerator as faRefrigerator
};
