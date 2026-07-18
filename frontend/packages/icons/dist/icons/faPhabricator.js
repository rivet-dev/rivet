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

// src/node_modules/@fortawesome/free-brands-svg-icons/faPhabricator.js
var require_faPhabricator = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faPhabricator.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "phabricator";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f3db";
    var svgPathData = "M331.1 262.1l-.1-13s21.7-19.8 21.1-21.2l-9.5-20c-.6-1.4-29.5-.5-29.5-.5l-9.4-9.3s.2-28.5-1.2-29.1l-20.1-9.2c-1.4-.6-20.7 21-20.7 21l-13.1-.2s-20.5-21.4-21.9-20.8l-20 8.3c-1.4 .5 .2 28.9 .2 28.9l-9.1 9.1s-29.2-.9-29.7 .4L160 226.3c-.6 1.4 21 21 21 21l.1 12.9S159.4 280 160 281.4l9.5 20c.6 1.4 29.5 .5 29.5 .5l9.4 9.3s-.2 31.8 1.2 32.3l20.1 8.3c1.4 .6 20.7-23.5 20.7-23.5l13.1 .2s20.5 23.8 21.8 23.3l20-7.5c1.4-.6-.2-32.1-.2-32.1l9.1-9.1s29.2 .9 29.7-.5l8.1-19.8c.7-1.1-20.9-20.7-20.9-20.7zm-44.9-8.7c.7 17.1-12.8 31.6-30.1 32.4s-32.1-12.5-32.8-29.6 12.8-31.6 30.1-32.3c17.3-.8 32.1 12.5 32.8 29.5zm201.2-37.9l-97-97-.1 .1c-75.1-73.3-195.4-72.8-269.8 1.6-50.9 51-27.8 27.9-95.7 95.3-22.3 22.3-22.3 58.7 0 81 69.9 69.4 46.4 46 97.4 97l.1-.1c75.1 73.3 195.4 72.9 269.8-1.6 51-50.9 27.9-27.9 95.3-95.3 22.3-22.3 22.3-58.7 0-81zM363.5 148.2a152.3 152.3 0 1 1 -215 215.6 152.3 152.3 0 1 1 215-215.6z";
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
    exports.faPhabricator = exports.definition;
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

// src/_entries/faPhabricator.js
var import_faPhabricator = __toESM(require_faPhabricator());
var export_faPhabricator = import_faPhabricator.definition;
export {
  export_faPhabricator as faPhabricator
};
