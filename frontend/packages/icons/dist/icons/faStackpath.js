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

// src/node_modules/@fortawesome/free-brands-svg-icons/faStackpath.js
var require_faStackpath = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faStackpath.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "stackpath";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f842";
    var svgPathData = "M244.6 232.4c0 8.5-4.3 20.5-21.3 20.5l-19.6 0 0-41.5 19.6 0c17.1 0 21.3 12.4 21.3 21zM448 32l0 448-448 0 0-448 448 0zM151.3 287.8c0-21.2-12.1-34.5-46.7-44.8-20.6-7.4-26-10.9-26-18.6s7-14.6 20.4-14.6c14.1 0 20.8 8.4 20.8 18.4l30.7 0 .2-.6c.5-19.6-15.1-41.6-51.1-41.6-23.4 0-52.6 10.8-52.6 38.3 0 19.4 9.2 31.3 50.7 44.4 17.3 6.2 21.9 10.4 21.9 19.5 0 15.2-19.1 14.2-19.5 14.2-20.4 0-25.7-9.1-25.7-21.9l-30.8 0-.2 .6c-.7 31.3 28.4 45.2 56.6 45.2 30 0 51.1-13.6 51.1-38.3zm125.4-55.6c0-25.3-18.4-45.5-53.4-45.5l-51.8 0 0 138.2 32.2 0 0-47.4 19.6 0c30.3 0 53.4-16 53.4-45.4zM297.9 325l49.1-138.2-31.1 0-47.9 138.2 29.9 0zM404.5 186.8l-31.1 0-47.9 138.2 29.9 0 49.1-138.2z";
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
    exports.faStackpath = exports.definition;
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

// src/_entries/faStackpath.js
var import_faStackpath = __toESM(require_faStackpath());
var export_faStackpath = import_faStackpath.definition;
export {
  export_faStackpath as faStackpath
};
