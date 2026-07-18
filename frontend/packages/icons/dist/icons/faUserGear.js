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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUserGear.js
var require_faUserGear = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUserGear.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-gear";
    var width = 640;
    var height = 512;
    var aliases = ["user-cog"];
    var unicode = "f4fe";
    var svgPathData = "M256.5 8a120 120 0 1 1 0 240 120 120 0 1 1 0-240zM226.7 304l59.4 0 1.5 0c-12.9 26.8-7.8 58.2 11.5 79.5-20.2 22.3-24.8 55.8-9.4 83.4l22.5 40.4c.9 1.6 1.9 3.2 2.9 4.7l-237 0c-16.4 0-29.7-13.3-29.7-29.7 0-98.5 79.8-178.3 178.3-178.3zm205.9-56.4c0-13.3 10.7-24 24-24l48 0c13.3 0 24 10.7 24 24l0 6.1c0 18.9 24.1 32.8 40.5 23.4l5-2.9c11.6-6.7 26.5-2.6 33 9.1l22.4 40.2c6.2 11.2 2.6 25.2-8.2 32l-4.7 2.9c-16.2 10.1-16.2 39.9 0 50.1l4.6 2.9c10.8 6.8 14.5 20.8 8.3 32L607 483.8c-6.5 11.7-21.4 15.9-33 9.1l-4.9-2.9c-16.4-9.5-40.5 4.5-40.5 23.4l0 6.1c0 13.3-10.7 24-24 24l-48 0c-13.3 0-24-10.7-24-24l0-5.9c0-19-24.2-33-40.7-23.5l-4.8 2.8c-11.6 6.7-26.4 2.6-33-9.1l-22.6-40.4c-6.2-11.2-2.6-25.3 8.3-32.1l4.4-2.7c16.3-10.1 16.3-40.1 0-50.2l-4.5-2.8c-10.9-6.8-14.5-20.9-8.3-32.1l22.5-40.3c6.5-11.7 21.4-15.8 32.9-9.1l4.8 2.8c16.5 9.5 40.7-4.5 40.7-23.5l0-5.9zm99.9 136.2a52 52 0 1 0 -104 0 52 52 0 1 0 104 0z";
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
    exports.faUserGear = exports.definition;
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

// src/_entries/faUserGear.js
var import_faUserGear = __toESM(require_faUserGear());
var export_faUserGear = import_faUserGear.definition;
export {
  export_faUserGear as faUserGear
};
