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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSourcetree.js
var require_faSourcetree = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSourcetree.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "sourcetree";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f7d3";
    var svgPathData = "M427.4 203c0-112.1-90.9-203-203-203-112.1-.2-203 90.6-203.2 202.6-.1 43 13.5 84.8 38.7 119.6s60.9 60.6 101.8 73.8l0 101.7c0 3.8 1.5 7.4 4.2 10.1s6.3 4.2 10.1 4.2l96.4 0c3.8 0 7.4-1.5 10.1-4.2s4.2-6.3 4.2-10.1l0-101.6c40.8-13.2 76.5-39 101.7-73.7s38.9-76.5 39-119.4zm-271.6 0c0-90.8 137.3-90.8 137.3 0-.1 89.9-137.3 91-137.3 0z";
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
    exports.faSourcetree = exports.definition;
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

// src/_entries/faSourcetree.js
var import_faSourcetree = __toESM(require_faSourcetree());
var export_faSourcetree = import_faSourcetree.definition;
export {
  export_faSourcetree as faSourcetree
};
