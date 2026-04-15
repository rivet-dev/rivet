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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRectanglesMixed.js
var require_faRectanglesMixed = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRectanglesMixed.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "rectangles-mixed";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e323";
    var svgPathData = "M0 80C0 53.5 21.5 32 48 32l224 0c26.5 0 48 21.5 48 48l0 128c0 26.5-21.5 48-48 48L48 256c-26.5 0-48-21.5-48-48L0 80zm384 0c0-26.5 21.5-48 48-48l96 0c26.5 0 48 21.5 48 48l0 256c0 26.5-21.5 48-48 48l-96 0c-26.5 0-48-21.5-48-48l0-256zM112 304l176 0c26.5 0 48 21.5 48 48l0 80c0 26.5-21.5 48-48 48l-176 0c-26.5 0-48-21.5-48-48l0-80c0-26.5 21.5-48 48-48z";
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
    exports.faRectanglesMixed = exports.definition;
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

// src/_entries/faRectanglesMixed.js
var import_faRectanglesMixed = __toESM(require_faRectanglesMixed());
var export_faRectanglesMixed = import_faRectanglesMixed.definition;
export {
  export_faRectanglesMixed as faRectanglesMixed
};
