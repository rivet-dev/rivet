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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHouseTree.js
var require_faHouseTree = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHouseTree.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "house-tree";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e1b3";
    var svgPathData = "M393.4 9.4c12.5-12.5 32.8-12.5 45.3 0l128 128c9.2 9.2 11.9 22.9 6.9 34.9s-16.6 19.8-29.6 19.8l-29.4 0L601 300c7.7 9.6 9.2 22.8 3.9 33.9S588.3 352 576 352l-29.4 0L633 460c7.7 9.6 9.2 22.8 3.9 33.9S620.3 512 608 512l-208 0c10-13.4 16-30 16-48l0-186.9c0-22.4-9.4-43.8-25.9-59L282.9 119.9 393.4 9.4zM0 277.1c0-13.5 5.6-26.3 15.6-35.4l144-132c18.4-16.8 46.5-16.8 64.9 0l144 132c9.9 9.1 15.6 21.9 15.6 35.4L384 464c0 26.5-21.5 48-48 48L48 512c-26.5 0-48-21.5-48-48L0 277.1zM144 296l0 48c0 13.3 10.7 24 24 24l48 0c13.3 0 24-10.7 24-24l0-48c0-13.3-10.7-24-24-24l-48 0c-13.3 0-24 10.7-24 24z";
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
    exports.faHouseTree = exports.definition;
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

// src/_entries/faHouseTree.js
var import_faHouseTree = __toESM(require_faHouseTree());
var export_faHouseTree = import_faHouseTree.definition;
export {
  export_faHouseTree as faHouseTree
};
