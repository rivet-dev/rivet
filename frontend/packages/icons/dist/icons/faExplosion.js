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

// src/node_modules/@fortawesome/free-solid-svg-icons/faExplosion.js
var require_faExplosion = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faExplosion.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "explosion";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e4e9";
    var svgPathData = "M256-32c13.3 0 24 10.7 24 24l0 48c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-48c0-13.3 10.7-24 24-24zm0 160c12.4 0 22.7 9.4 23.9 21.7L293.4 291.6 467.6 11.3c6.7-10.7 20.5-14.5 31.7-8.5s15.8 19.5 10.6 31L372.8 338.6c2.2 2.3 4.3 4.7 6.3 7.1l97.2-54.7c10.5-5.9 23.6-3.1 30.9 6.4s6.3 23-2.2 31.5l-87 87-71.4 0c-13.2-37.3-48.7-64-90.5-64s-77.4 26.7-90.5 64l-79.6 0-75.5-52.3C.6 357-2.8 344.1 2.4 333.5s17.4-15.9 28.7-12.4l97.2 30.4c3-3.9 6.1-7.6 9.4-11.3L75.4 236.3c-6.1-10.1-3.9-23.1 5.1-30.7s22.2-7.5 31.1 .1L214 293.6c1.5-.4 3-.8 4.5-1.1l13.6-142.7c1.2-12.3 11.5-21.7 23.9-21.7zM32 464l456 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L24 512c-13.3 0-24-10.7-24-24s10.7-24 24-24l8 0z";
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
    exports.faExplosion = exports.definition;
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

// src/_entries/faExplosion.js
var import_faExplosion = __toESM(require_faExplosion());
var export_faExplosion = import_faExplosion.definition;
export {
  export_faExplosion as faExplosion
};
