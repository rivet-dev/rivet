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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceAwesome.js
var require_faFaceAwesome = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceAwesome.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-awesome";
    var width = 512;
    var height = 512;
    var aliases = ["gave-dandy"];
    var unicode = "e409";
    var svgPathData = "M0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zm203.5-95.6c-5.6 9.4-15.8 15.6-27.5 15.6c-17.7 0-32-14.3-32-32c0-9.1 3.8-17.4 10-23.2c-3.3-.5-6.6-.8-10-.8c-35.3 0-64 28.7-64 64l0 10.3c0 16.4 13.3 29.7 29.7 29.7l68.6 0c16.4 0 29.7-13.3 29.7-29.7l0-10.3c0-8.3-1.6-16.3-4.5-23.6zM384 176c-17.7 0-32-14.3-32-32c0-8.5 3.3-16.3 8.8-22c-5.4-1.3-11-2-16.8-2c-39.8 0-72 32.2-72 72l0 7.5c0 13.5 11 24.5 24.5 24.5l95.1 0c13.5 0 24.5-11 24.5-24.5l0-7.5c0-10.4-2.2-20.2-6.1-29.1c-5.8 8-15.2 13.1-25.9 13.1zM245.5 447.8c2.7 .1 5.4 .2 8.1 .2l4.9 0c55.1 0 105.1-26.1 137-67.5c0 0 0 0 0 0c15.7-20.5 27-44.6 32.4-71.3c2.2-11-6.2-21.2-17.4-21.2c0 0 0 0 0 0l-308.8 0s0 0 0 0c-11.2 0-19.6 10.2-17.4 21.2c15.6 78.1 82.4 135 161.2 138.6zm-5.4-32.5c1.5-44.1 37.8-79.4 82.2-79.4c19.9 0 38.1 7 52.3 18.7C349 392.1 306.1 416 258.4 416l-4.9 0c-4.6 0-9.1-.2-13.5-.6z";
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
    exports.faFaceAwesome = exports.definition;
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

// src/_entries/faGaveDandy.js
var import_faFaceAwesome = __toESM(require_faFaceAwesome());
var export_faGaveDandy = import_faFaceAwesome.definition;
export {
  export_faGaveDandy as faGaveDandy
};
