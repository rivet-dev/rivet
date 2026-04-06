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

// src/node_modules/@fortawesome/free-brands-svg-icons/faLinktree.js
var require_faLinktree = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faLinktree.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "linktree";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e7d8";
    var svgPathData = "M16 173.4l125.8 0-89.4-85.4 49.5-51 85.2 87.8 0-124.8 73.9 0 0 124.8 85.2-87.6 49.4 50.8-89.4 85.2 125.7 0 0 70.5-126.4 0 90 87.6-49.3 49.8-122.2-123-122.2 123-49.5-49.6 90-87.6-126.4 0 0-70.5zM186.9 344.8l73.9 0 0 167.2-73.9 0 0-167.2z";
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
    exports.faLinktree = exports.definition;
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

// src/_entries/faLinktree.js
var import_faLinktree = __toESM(require_faLinktree());
var export_faLinktree = import_faLinktree.definition;
export {
  export_faLinktree as faLinktree
};
