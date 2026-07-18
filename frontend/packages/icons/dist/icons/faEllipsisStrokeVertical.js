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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faEllipsisStrokeVertical.js
var require_faEllipsisStrokeVertical = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faEllipsisStrokeVertical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ellipsis-stroke-vertical";
    var width = 192;
    var height = 512;
    var aliases = ["ellipsis-v-alt"];
    var unicode = "f39c";
    var svgPathData = "M96 72a24 24 0 1 0 0 48 24 24 0 1 0 0-48zm0 96A72 72 0 1 1 96 24a72 72 0 1 1 0 144zm24 88a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm-96 0a72 72 0 1 1 144 0A72 72 0 1 1 24 256zm96 160a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm-96 0a72 72 0 1 1 144 0A72 72 0 1 1 24 416z";
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
    exports.faEllipsisStrokeVertical = exports.definition;
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

// src/_entries/faEllipsisStrokeVertical.js
var import_faEllipsisStrokeVertical = __toESM(require_faEllipsisStrokeVertical());
var export_faEllipsisStrokeVertical = import_faEllipsisStrokeVertical.definition;
export {
  export_faEllipsisStrokeVertical as faEllipsisStrokeVertical
};
