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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleDownLeft.js
var require_faCircleDownLeft = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleDownLeft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-down-left";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e107";
    var svgPathData = "M256 512A256 256 0 1 1 256 0a256 256 0 1 1 0 512zM160 198.6L160 336c0 8.8 7.2 16 16 16l137.4 0c12.5 0 22.6-10.1 22.6-22.6c0-6-2.4-11.8-6.6-16L296 280l66.3-66.3c3.6-3.6 5.7-8.5 5.7-13.7s-2-10-5.7-13.7l-36.7-36.7C322 146 317.1 144 312 144s-10 2-13.7 5.7L232 216l-33.4-33.4c-4.2-4.2-10-6.6-16-6.6c-12.5 0-22.6 10.1-22.6 22.6z";
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
    exports.faCircleDownLeft = exports.definition;
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

// src/_entries/faCircleDownLeft.js
var import_faCircleDownLeft = __toESM(require_faCircleDownLeft());
var export_faCircleDownLeft = import_faCircleDownLeft.definition;
export {
  export_faCircleDownLeft as faCircleDownLeft
};
