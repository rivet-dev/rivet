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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSkiJumping.js
var require_faPersonSkiJumping = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonSkiJumping.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-ski-jumping";
    var width = 512;
    var height = 512;
    var aliases = ["ski-jump"];
    var unicode = "f7c7";
    var svgPathData = "M352 48a48 48 0 1 1 96 0 48 48 0 1 1 -96 0zM128 128c0-17.7 14.3-32 32-32l144.5 0c46 0 65.7 58.4 29 86.2L230.1 260.8l-6.4 7.5c-2.4 2.8-4.4 6.1-5.7 9.6L179.5 380.6 433.7 249.1c18.6-9.6 30.3-28.8 30.3-49.7l0-7.3c0-13.3 10.7-24 24-24s24 10.7 24 24l0 7.3c0 38.9-21.7 74.5-56.2 92.4L35 509.3C23.3 515.4 8.8 510.8 2.7 499s-1.5-26.3 10.3-32.3l111.2-57.5c-10.7-8.4-15.2-23-10.2-36.4l44-117.4c3.9-10.5 9.7-20.2 17-28.8L232.1 160 160 160c-17.7 0-32-14.3-32-32z";
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
    exports.faPersonSkiJumping = exports.definition;
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

// src/_entries/faSkiJump.js
var import_faPersonSkiJumping = __toESM(require_faPersonSkiJumping());
var export_faSkiJump = import_faPersonSkiJumping.definition;
export {
  export_faSkiJump as faSkiJump
};
