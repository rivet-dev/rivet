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

// src/node_modules/@fortawesome/free-solid-svg-icons/faGolfBallTee.js
var require_faGolfBallTee = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faGolfBallTee.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "golf-ball-tee";
    var width = 384;
    var height = 512;
    var aliases = ["golf-ball"];
    var unicode = "f450";
    var svgPathData = "M298.5 384.1c12.1 1.2 21.5 11.5 21.5 23.9s-9.4 22.6-21.5 23.9l-2.5 .1-80 0 0 88c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-88-80 0c-13.3 0-24-10.7-24-24 0-12.4 9.4-22.6 21.5-23.9l2.5-.1 208 0 2.5 .1zM192 0c106 0 192 86 192 192 0 57.4-25.2 108.8-65.1 144L65.1 336C25.2 300.8 0 249.4 0 192 0 86 86 0 192 0zm32 240c-7.7 0-14.2 5.5-15.7 12.8l-.6 6.4c-1.3 6.3-6.2 11.2-12.5 12.5l-6.4 .6c-7.3 1.5-12.8 8-12.8 15.7 0 8.8 7.2 16 16 16 26.5 0 48-21.5 48-48 0-8.8-7.2-16-16-16zm64-64c-7.7 0-14.2 5.5-15.7 12.8l-.6 6.4c-1.3 6.3-6.2 11.2-12.5 12.5l-6.4 .6c-7.3 1.5-12.8 8-12.8 15.7 0 8.8 7.2 16 16 16 26.5 0 48-21.5 48-48 0-8.8-7.2-16-16-16zm-80-16c-7.7 0-14.2 5.5-15.7 12.8l-.6 6.4c-1.3 6.3-6.2 11.2-12.5 12.5l-6.4 .6c-7.3 1.5-12.8 8-12.8 15.7 0 8.8 7.2 16 16 16 26.5 0 48-21.5 48-48 0-8.8-7.2-16-16-16z";
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
    exports.faGolfBallTee = exports.definition;
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

// src/_entries/faGolfBall.js
var import_faGolfBallTee = __toESM(require_faGolfBallTee());
var export_faGolfBall = import_faGolfBallTee.definition;
export {
  export_faGolfBall as faGolfBall
};
