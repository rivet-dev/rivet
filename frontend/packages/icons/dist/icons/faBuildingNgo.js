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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBuildingNgo.js
var require_faBuildingNgo = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBuildingNgo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "building-ngo";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e4d7";
    var svgPathData = "M128 0C92.7 0 64 28.7 64 64l0 384c0 35.3 28.7 64 64 64l48 0 0-112c0-35.3 28.7-64 64-64l208 0 0-272c0-35.3-28.7-64-64-64L128 0zm32 112c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-32zM304 96l32 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16zM160 240c0-8.8 7.2-16 16-16l32 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-32zm144-16l32 0c8.8 0 16 7.2 16 16l0 32c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-32c0-8.8 7.2-16 16-16zM520 380c-24.3 0-44 19.7-44 44l0 80c0 24.3 19.7 44 44 44l16 0c24.3 0 44-19.7 44-44l0-80c0-24.3-19.7-44-44-44l-16 0zm-4 44c0-2.2 1.8-4 4-4l16 0c2.2 0 4 1.8 4 4l0 80c0 2.2-1.8 4-4 4l-16 0c-2.2 0-4-1.8-4-4l0-80zm-168 0l0 80c0 24.3 19.7 44 44 44l16 0c24.3 0 44-19.7 44-44l0-24c0-11-9-20-20-20l-8 0c-11 0-20 9-20 20 0 6.5 3.1 12.4 8 16l0 8c0 2.2-1.8 4-4 4l-16 0c-2.2 0-4-1.8-4-4l0-80c0-2.2 1.8-4 4-4l20.4 0c1.9 9.1 9.9 16 19.6 16 11 0 20-9 20-20 0-19.9-16.1-36-36-36l-24 0c-24.3 0-44 19.7-44 44zm-90.1-32.9c-4.1-8.3-13.5-12.7-22.5-10.5S220 390.7 220 400l0 128c0 11 9 20 20 20s20-9 20-20l0-43.3 26.1 52.2c4.1 8.3 13.5 12.7 22.5 10.5S324 537.3 324 528l0-128c0-11-9-20-20-20s-20 9-20 20l0 43.3-26.1-52.2z";
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
    exports.faBuildingNgo = exports.definition;
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

// src/_entries/faBuildingNgo.js
var import_faBuildingNgo = __toESM(require_faBuildingNgo());
var export_faBuildingNgo = import_faBuildingNgo.definition;
export {
  export_faBuildingNgo as faBuildingNgo
};
