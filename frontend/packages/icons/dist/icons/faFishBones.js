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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFishBones.js
var require_faFishBones = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFishBones.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fish-bones";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e304";
    var svgPathData = "M87.1 148.2c5.6 4.6 8.9 11.4 8.9 18.7L96 208s0 0 0 0l0 24 40 0 0-72c0-13.3 10.7-24 24-24s24 10.7 24 24l0 72 48 0 0-72c0-13.3 10.7-24 24-24s24 10.7 24 24l0 72 48 0 0-72c0-13.3 10.7-24 24-24s24 10.7 24 24l0 72 40 0 0-101.1c0-16.9 17.1-28.4 31.8-20c16 9.1 30.6 19.7 43.7 30.7c39.1 33 66.9 72.4 81 99.8c4.7 9.2 4.7 20.1 0 29.3c-14.1 27.4-41.9 66.8-81 99.8c-13.1 11-27.7 21.6-43.7 30.7c-14.7 8.4-31.8-3.1-31.8-20L416 280l-40 0 0 72c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-72-48 0 0 72c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-72-48 0 0 72c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-72-40 0 0 24s0 0 0 0l0 41.1c0 7.2-3.3 14.1-8.9 18.7l-48 38.9c-8.8 7.1-21.3 7.1-30.1 .1s-11.5-19.2-6.6-29.3L54.9 266.6c3.3-6.7 3.3-14.5 0-21.2L2.5 138.6c-5-10.1-2.2-22.3 6.6-29.3s21.3-7 30.1 .1l48 38.9z";
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
    exports.faFishBones = exports.definition;
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

// src/_entries/faFishBones.js
var import_faFishBones = __toESM(require_faFishBones());
var export_faFishBones = import_faFishBones.definition;
export {
  export_faFishBones as faFishBones
};
