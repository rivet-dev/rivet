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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserGroupCrown.js
var require_faUserGroupCrown = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserGroupCrown.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-group-crown";
    var width = 640;
    var height = 512;
    var aliases = ["users-crown"];
    var unicode = "f6a5";
    var svgPathData = "M144 144l0-16 160 0 0 16c0 44.2-35.8 80-80 80s-80-35.8-80-80zm-.1-124.9c-.3-.2-.6-.4-.8-.6L140 16 124.4 3.5C121.5 1.2 118 0 114.4 0L112 0c-8.8 0-16 7.2-16 16l0 23 0 3.2c0 0 0 .1 0 .1L96 144c0 70.7 57.3 128 128 128s128-57.3 128-128l0-101.7c0 0 0-.1 0-.1l0-3.2 0-23c0-8.8-7.2-16-16-16l-2.4 0c-3.6 0-7.2 1.2-10 3.5L308 16l-3 2.4c-.3 .2-.6 .4-.8 .6l-.1 .1c-10.2 7.5-23.8 8.3-34.9 2L238.9 4c-4.6-2.6-9.7-4-14.9-4s-10.4 1.4-14.9 4L178.9 21.2c-11 6.3-24.7 5.5-34.9-2l-.1-.1zM168 304C75.2 304 0 379.2 0 472l0 8c0 17.7 14.3 32 32 32l384 0c17.7 0 32-14.3 32-32l0-8c0-92.8-75.2-168-168-168l-112 0zm264-48c61.9 0 112-50.1 112-112s-50.1-112-112-112c-17.2 0-33.5 3.9-48 10.8L384 144c0 5.4-.3 10.7-.8 16c-2.4 24.3-10.3 47.1-22.4 67c-.5 .8-1 1.6-1.4 2.3C379 245.9 404.3 256 432 256zm39.4 256l137.8 0c17 0 30.7-13.8 30.7-30.7C640 392.2 567.8 320 478.7 320l-61.4 0c-2.4 0-4.8 .1-7.1 .2C452.9 356.8 480 411.3 480 472l0 8c0 11.7-3.1 22.6-8.6 32z";
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
    exports.faUserGroupCrown = exports.definition;
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

// src/_entries/faUserGroupCrown.js
var import_faUserGroupCrown = __toESM(require_faUserGroupCrown());
var export_faUserGroupCrown = import_faUserGroupCrown.definition;
export {
  export_faUserGroupCrown as faUserGroupCrown
};
