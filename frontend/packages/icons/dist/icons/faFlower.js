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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFlower.js
var require_faFlower = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFlower.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "flower";
    var width = 448;
    var height = 512;
    var aliases = [127804, 10047];
    var unicode = "f7ff";
    var svgPathData = "M128 480c38.2 0 72.5-16.8 96-43.3c23.5 26.6 57.8 43.3 96 43.3c70.7 0 128-57.3 128-128c0-38.2-16.8-72.5-43.3-96c26.6-23.5 43.3-57.8 43.3-96c0-70.7-57.3-128-128-128c-38.2 0-72.5 16.8-96 43.3C200.5 48.8 166.2 32 128 32C57.3 32 0 89.3 0 160c0 38.2 16.8 72.5 43.3 96C16.8 279.5 0 313.8 0 352c0 70.7 57.3 128 128 128zm96-304a80 80 0 1 1 0 160 80 80 0 1 1 0-160z";
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
    exports.faFlower = exports.definition;
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

// src/_entries/faFlower.js
var import_faFlower = __toESM(require_faFlower());
var export_faFlower = import_faFlower.definition;
export {
  export_faFlower as faFlower
};
