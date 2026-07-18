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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleArrowUpRight.js
var require_faCircleArrowUpRight = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleArrowUpRight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-arrow-up-right";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0fc";
    var svgPathData = "M0 256a256 256 0 1 0 512 0A256 256 0 1 0 0 256zm368 64c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-94.1L185 361c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9l135-135L184 192c-13.3 0-24-10.7-24-24s10.7-24 24-24l160 0c13.3 0 24 10.7 24 24l0 152z";
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
    exports.faCircleArrowUpRight = exports.definition;
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

// src/_entries/faCircleArrowUpRight.js
var import_faCircleArrowUpRight = __toESM(require_faCircleArrowUpRight());
var export_faCircleArrowUpRight = import_faCircleArrowUpRight.definition;
export {
  export_faCircleArrowUpRight as faCircleArrowUpRight
};
