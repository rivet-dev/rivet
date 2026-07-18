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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTankWater.js
var require_faTankWater = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTankWater.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tank-water";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e452";
    var svgPathData = "M285.9 96l-22-40.3L251 32l133 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96C0 60.7 28.7 32 64 32l133 0L184.1 55.7 162.1 96 64 96l0 201.9c8.3-3 16.2-6.8 23.3-10.8c6.9-3.9 13.1-8.2 18.9-12.7c12.9-10 30.6-10 43.4 0c5.9 4.6 12 8.8 18.9 12.7c15.8 9 35.5 16.8 55.2 16.9l.4 0c19.6 0 39.3-7.9 55.2-16.9c6.9-3.9 13.1-8.2 18.9-12.7c12.9-10 30.6-10 43.4 0c5.9 4.6 12 8.8 19 12.8c7.1 4 15 7.9 23.4 10.8l0-202-98.1 0zM175 139.2L212.2 71c2.4-4.3 6.9-7 11.8-7s9.5 2.7 11.8 7L273 139.2c4.6 8.4 7 17.9 7 27.5l0 1.4c0 30.9-25.1 56-56 56s-56-25.1-56-56l0-1.4c0-9.6 2.4-19 7-27.5z";
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
    exports.faTankWater = exports.definition;
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

// src/_entries/faTankWater.js
var import_faTankWater = __toESM(require_faTankWater());
var export_faTankWater = import_faTankWater.definition;
export {
  export_faTankWater as faTankWater
};
