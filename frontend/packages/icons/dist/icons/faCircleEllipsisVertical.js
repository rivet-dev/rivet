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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleEllipsisVertical.js
var require_faCircleEllipsisVertical = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleEllipsisVertical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-ellipsis-vertical";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e10b";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zm0-288a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm-32-64a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm32 160a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faCircleEllipsisVertical = exports.definition;
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

// src/_entries/faCircleEllipsisVertical.js
var import_faCircleEllipsisVertical = __toESM(require_faCircleEllipsisVertical());
var export_faCircleEllipsisVertical = import_faCircleEllipsisVertical.definition;
export {
  export_faCircleEllipsisVertical as faCircleEllipsisVertical
};
