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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleDownRight.js
var require_faCircleDownRight = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleDownRight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-down-right";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e108";
    var svgPathData = "M256 0a256 256 0 1 0 0 512A256 256 0 1 0 256 0zm96 198.6L352 336c0 8.8-7.2 16-16 16l-137.4 0c-12.5 0-22.6-10.1-22.6-22.6c0-6 2.4-11.8 6.6-16L216 280l-66.3-66.3C146 210 144 205.1 144 200s2-10 5.7-13.7l36.7-36.7c3.6-3.6 8.5-5.7 13.7-5.7s10 2 13.7 5.7L280 216l33.4-33.4c4.2-4.2 10-6.6 16-6.6c12.5 0 22.6 10.1 22.6 22.6z";
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
    exports.faCircleDownRight = exports.definition;
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

// src/_entries/faCircleDownRight.js
var import_faCircleDownRight = __toESM(require_faCircleDownRight());
var export_faCircleDownRight = import_faCircleDownRight.definition;
export {
  export_faCircleDownRight as faCircleDownRight
};
